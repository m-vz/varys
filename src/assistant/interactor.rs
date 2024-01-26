use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use log::{debug, error, info, warn};
use sqlx::PgPool;

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

const SILENCE_DURATION: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct InteractorBuilder {
    pub interface: String,
    pub voice: String,
    pub sensitivity: f32,
    pub model: Model,
    pub data_dir: PathBuf,
}

impl InteractorBuilder {
    /// Create an interactor builder to build an interactor and all it's components.
    ///
    /// # Arguments
    ///
    /// * `interface`: The interface to create the sniffer on.
    /// * `voice`: The voice to use for the speaker.
    /// * `sensitivity`: The sensitivity of the listener.
    /// * `model`: The model to use for the recogniser.
    pub fn new(
        interface: String,
        voice: String,
        sensitivity: f32,
        model: Model,
        data_dir: PathBuf,
    ) -> InteractorBuilder {
        InteractorBuilder {
            interface,
            voice,
            sensitivity,
            model,
            data_dir,
        }
    }

    /// Build an interactor.
    ///
    /// This will create a [`Listener`], a [`Sniffer`], a [`Speaker`] and a [`Recogniser`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::path::PathBuf;
    /// # use varys::assistant::interactor::InteractorBuilder;
    /// # use varys::recognise::Model;
    /// let mut interactor = InteractorBuilder::new(
    ///     "en0".to_string(),
    ///     "Ava".to_string(),
    ///     0.01,
    ///     Model::Large,
    ///     PathBuf::from("./data")
    /// ).build().unwrap();
    /// ```
    pub fn build(self) -> Result<Interactor, Error> {
        let config = InteractorConfig {
            interface: self.interface.clone(),
            voice: self.voice.clone(),
            sensitivity: self.sensitivity.to_string(),
            model: self.model.to_string(),
        };

        Ok(Interactor {
            listener: Listener::new()?,
            sniffer: Sniffer::from(sniff::device_by_name(self.interface.as_str())?),
            speaker: Speaker::with_voice(self.voice.as_str())?,
            sensitivity: self.sensitivity,
            recogniser: Recogniser::with_model(self.model)?,
            config,
            data_dir: self.data_dir,
        })
    }
}

pub struct Interactor {
    listener: Listener,
    sniffer: Sniffer,
    speaker: Speaker,
    sensitivity: f32,
    recogniser: Recogniser,
    config: InteractorConfig,
    data_dir: PathBuf,
}

impl Interactor {
    /// Set up a database connection and begin a new session of interactions.
    ///
    /// Returns a [`RunningInteractor`] that can be started.
    pub async fn begin_session(self) -> Result<RunningInteractor, Error> {
        // connect to database and start session
        let database_pool = database::connect().await?;
        let mut session = Session::create(&database_pool, &self.config).await?;

        // create and store session path
        let session_path = self
            .data_dir
            .join(Path::new(&format!("sessions/session_{}", session.id)));
        fs::create_dir_all(&session_path)?;
        session.data_dir = Some(session_path.to_string_lossy().to_string());
        session.update(&database_pool).await?;

        info!("Starting session {}", session.id);
        debug!("Storing data files at {}", session_path.to_string_lossy());

        Ok(RunningInteractor {
            interactor: self,
            database_pool,
            session,
            session_path,
        })
    }
}

pub struct RunningInteractor {
    interactor: Interactor,
    database_pool: PgPool,
    session: Session,
    session_path: PathBuf,
}

impl RunningInteractor {
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
    /// # use varys::assistant::interactor::InteractorBuilder;
    /// # use varys::database::query::Query;
    /// # use varys::recognise::Model;
    /// let mut interactor = InteractorBuilder::new(
    ///     "ap1".to_string(),
    ///     "Ava".to_string(),
    ///     0.01,
    ///     Model::Large,
    ///     PathBuf::from("./data")
    /// )
    /// .build()
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
    ///     .begin_session()
    ///     .await
    ///     .unwrap()
    ///     .start(&queries)
    ///     .await
    ///     .unwrap();
    /// #     })
    /// ```
    pub async fn start(mut self, queries: &Vec<Query>) -> Result<Interactor, Error> {
        for query in queries {
            if let Err(error) = self.interaction(query).await {
                error!("An interaction did not complete successfully: {error}");
            }
        }

        self.session.complete(&self.database_pool).await?;

        Ok(self.interactor)
    }

    async fn interaction(&mut self, query: &Query) -> Result<(), Error> {
        info!("Starting interaction with \"{query}\"");

        // notify monitoring about interaction
        if let Err(error) = monitoring::ping(&format!("Interaction started: {query}")).await {
            warn!("Failed to notify monitoring about interaction: {}", error);
        }

        // prepare the interaction
        let mut interaction =
            Interaction::create(&self.database_pool, &self.session, query).await?;

        // start the sniffer
        let capture_path = self
            .session_path
            .join(capture_file_name(&self.session, &interaction));
        let sniffer_instance = self.interactor.sniffer.start(&capture_path)?;

        // say the query
        interaction.query_duration = Some(self.interactor.speaker.say(&query.text, true)?);
        interaction.update(&self.database_pool).await?;

        // record the response
        let mut audio = self
            .interactor
            .listener
            .record_until_silent(SILENCE_DURATION, self.interactor.sensitivity)?;
        interaction.response_duration = Some(audio.duration());
        let audio_path = self
            .session_path
            .join(audio_file_name(&self.session, &interaction));
        file::audio::write_audio(&audio_path, &audio)?;
        interaction.response_file = Some(file::file_name_or_full(&audio_path));

        // recognise the response
        interaction.response = Some(self.interactor.recogniser.recognise(&mut audio)?);
        interaction.update(&self.database_pool).await?;

        // finish the sniffer
        info!("{}", sniffer_instance.stop()?);
        interaction.capture_file = Some(file::file_name_or_full(&capture_path));

        // finish the interaction
        interaction.complete(&self.database_pool).await?;

        Ok(())
    }
}

fn audio_file_name(session: &Session, interaction: &Interaction) -> PathBuf {
    data_file_name(session, interaction, "audio", "opus")
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
