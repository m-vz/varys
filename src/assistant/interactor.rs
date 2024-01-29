use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use log::{debug, error, info, warn};
use sqlx::PgPool;

use crate::assistant::VoiceAssistant;
use crate::database::interaction::Interaction;
use crate::database::interactor_config::InteractorConfig;
use crate::database::query::Query;
use crate::database::session::Session;
use crate::error::Error;
use crate::listen::Listener;
use crate::recognise::{Model, Recogniser};
use crate::sniff::Sniffer;
use crate::speak::Speaker;
use crate::{database, file, monitoring, sniff};

/// How long there must be silence for the recording to be stopped.
const SILENCE_AFTER_RECORDING: Duration = Duration::from_secs(2);
/// How long there must be silence before the next interaction is begun
pub const MINIMUM_SILENCE_BETWEEN_INTERACTIONS: Duration = Duration::from_secs(4);

pub struct Interactor {
    pub recogniser: Recogniser,
    pub listener: Listener,
    pub sniffer: Sniffer,
    interface: String,
    pub speaker: Speaker,
    pub voices: VecDeque<String>,
    pub sensitivity: f32,
    model: Model,
    pub data_dir: PathBuf,
}

impl Interactor {
    /// Create an interactor.
    ///
    /// This will create a [`Recogniser`].
    ///
    /// # Arguments
    ///
    /// * `interface`: The interface to create the sniffer on.
    /// * `voices`: The voices to use for the speaker.
    /// * `sensitivity`: The sensitivity of the listener.
    /// * `model`: The model to use for the recogniser.
    /// * `data_dir`: The path to the data directory.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::path::PathBuf;
    /// # use varys::assistant::interactor::Interactor;
    /// # use varys::recognise::Model;
    /// let mut interactor = Interactor::new(
    ///     "en0".to_string(),
    ///     vec!["Ava".to_string()],
    ///     0.01,
    ///     Model::Large,
    ///     PathBuf::from("./data")
    /// ).unwrap();
    /// ```
    pub fn new(
        interface: String,
        voices: Vec<String>,
        sensitivity: f32,
        model: Model,
        data_dir: PathBuf,
    ) -> Result<Interactor, Error> {
        Ok(Interactor {
            recogniser: Recogniser::with_model(model)?,
            listener: Listener::new()?,
            sniffer: Sniffer::from(sniff::device_by_name(interface.as_str())?),
            interface,
            speaker: Speaker::new()?,
            voices: voices.into(),
            sensitivity,
            model,
            data_dir,
        })
    }

    /// Set up a database connection and begin a new session of interactions.
    ///
    /// This will create a [`Listener`], a [`Sniffer`], a [`Speaker`] and use the existing [`Recogniser`].
    ///
    /// Returns a [`InteractorInstance`] that can be started.
    pub async fn begin_session<A: VoiceAssistant>(
        mut self,
        assistant: &A,
    ) -> Result<InteractorInstance<A>, Error> {
        // choose next voice and re-queue it
        let voice = self.voices.pop_front().ok_or(Error::NoVoiceProvided)?;
        self.voices.push_back(voice.clone());
        self.speaker.set_voice(&voice)?;

        // connect to database and start session
        let database_pool = database::connect().await?;
        let mut session = Session::create(
            &database_pool,
            &InteractorConfig {
                interface: self.interface.to_string(),
                voice: voice.clone(),
                sensitivity: self.sensitivity.to_string(),
                model: self.model.to_string(),
            },
        )
        .await?;

        // create and store session path
        let session_path = self
            .data_dir
            .join(Path::new(&format!("sessions/session_{}", session.id)));
        fs::create_dir_all(&session_path)?;
        session.data_dir = Some(session_path.to_string_lossy().to_string());
        session.update(&database_pool).await?;
        debug!("Storing data files at {}", session_path.to_string_lossy());

        Ok(InteractorInstance {
            interactor: self,
            assistant,
            database_pool,
            session,
            session_path,
        })
    }
}

pub struct InteractorInstance<'a, A: VoiceAssistant> {
    interactor: Interactor,
    assistant: &'a A,
    database_pool: PgPool,
    session: Session,
    session_path: PathBuf,
}

impl<'a, A: VoiceAssistant> InteractorInstance<'a, A> {
    /// Start the prepared session with a list of queries.
    ///
    /// # Arguments
    ///
    /// * `queries`: The queries to ask during this session.
    ///
    /// Returns an [`Interactor`] with which a new session can be begun.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use varys::assistant;
    /// # use varys::assistant::interactor::Interactor;
    /// # use varys::database::query::Query;
    /// # use varys::recognise::Model;
    /// let mut interactor = Interactor::new(
    ///     "en0".to_string(),
    ///     vec!["Ava".to_string()],
    ///     0.01,
    ///     Model::Large,
    ///     PathBuf::from("./data")
    /// )
    /// .unwrap();
    /// let queries = vec![
    ///     Query {
    ///         text: "How are you?".to_string(),
    ///         category: "greeting".to_string(),
    ///     },
    ///     Query {
    ///         text: "What is your name?".to_string(),
    ///         category: "greeting".to_string(),
    ///     },
    /// ];
    /// # tokio::runtime::Builder::new_current_thread()
    /// #     .enable_all()
    /// #     .build()
    /// #     .unwrap()
    /// #     .block_on(async {
    /// interactor
    ///     .begin_session(&assistant::from("Siri"))
    ///     .await
    ///     .unwrap()
    ///     .start(&queries)
    ///     .await
    ///     .unwrap();
    /// #     })
    /// ```
    pub async fn start(mut self, queries: &Vec<Query>) -> Result<Interactor, Error> {
        info!("Starting {}", self.session);

        for query in queries {
            // notify monitoring about interaction
            if let Err(error) = monitoring::ping(&format!("Interaction started: {query}")).await {
                warn!("Failed to notify monitoring about interaction: {}", error);
            }

            // start the interaction
            if let Err(error) = self.interaction(query).await {
                error!("An interaction did not complete successfully: {error}");

                if let Error::RecordingTimeout = error {
                    self.assistant.reset_assistant(&mut self.interactor)?;
                }
            }

            // wait for silence to finish the interaction
            self.interactor.listener.wait_until_silent(
                MINIMUM_SILENCE_BETWEEN_INTERACTIONS,
                self.interactor.sensitivity,
            )?;

            // make sure the assistant is not waiting for an answer currently
            self.assistant.stop_assistant(&mut self.interactor)?;
        }

        self.session.complete(&self.database_pool).await?;

        Ok(self.interactor)
    }

    async fn interaction(&mut self, query: &Query) -> Result<(), Error> {
        info!("Starting interaction with \"{query}\"");

        // prepare the interaction
        let mut interaction =
            Interaction::create(&self.database_pool, &self.session, query).await?;
        let capture_path = self
            .session_path
            .join(capture_file_name(&self.session, &interaction));
        let query_audio_path =
            self.session_path
                .join(audio_file_name(&self.session, &interaction, "query"));
        let response_audio_path =
            self.session_path
                .join(audio_file_name(&self.session, &interaction, "response"));

        // start the sniffer
        let sniffer_instance = self.interactor.sniffer.start(&capture_path)?;

        // begin recording the query
        let query_instance = self.interactor.listener.start()?;

        // say the query
        interaction.query_duration = Some(self.interactor.speaker.say(&query.text, true)?);

        // stop recording the query
        let query_audio = query_instance.stop()?;

        file::audio::write_audio(&query_audio_path, &query_audio)?;
        interaction.query_file = Some(file::file_name_or_full(&query_audio_path));
        interaction.update(&self.database_pool).await?;

        // record the response
        let mut response_audio = self
            .interactor
            .listener
            .record_until_silent(SILENCE_AFTER_RECORDING, self.interactor.sensitivity)?;

        interaction.response_duration = Some(response_audio.duration_ms());
        file::audio::write_audio(&response_audio_path, &response_audio)?;
        interaction.response_file = Some(file::file_name_or_full(&response_audio_path));
        interaction.update(&self.database_pool).await?;

        // finish the sniffer
        let stats = sniffer_instance.stop()?;

        info!("{stats}");
        interaction.capture_file = Some(file::file_name_or_full(&capture_path));
        interaction.update(&self.database_pool).await?;

        // recognise the response
        let response = self.interactor.recogniser.recognise(&mut response_audio)?;
        interaction.response = Some(response.clone());
        interaction.update(&self.database_pool).await?;

        // finish the interaction
        interaction.complete(&self.database_pool).await?;

        Ok(())
    }
}

fn audio_file_name(session: &Session, interaction: &Interaction, prefix: &str) -> PathBuf {
    data_file_name(session, interaction, &format!("{prefix}-audio"), "opus")
}

fn capture_file_name(session: &Session, interaction: &Interaction) -> PathBuf {
    data_file_name(session, interaction, "capture", "pcap")
}

fn data_file_name(
    session: &Session,
    interaction: &Interaction,
    data_type: &str,
    file_type: &str,
) -> PathBuf {
    PathBuf::from(format!(
        "s{}i{}-{}-{}.{}",
        session.id,
        interaction.id,
        data_type,
        Utc::now().format("%Y-%m-%d-%H-%M-%S-%f"),
        file_type,
    ))
}
