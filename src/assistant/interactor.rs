use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use log::{debug, error, info, warn};
use rand::prelude::SliceRandom;
use sqlx::PgPool;

use crate::assistant::VoiceAssistant;
use crate::database::interaction::Interaction;
use crate::database::interactor_config::InteractorConfig;
use crate::database::query::Query;
use crate::database::session::Session;
use crate::error::Error;
use crate::listen::audio::AudioData;
use crate::listen::Listener;
use crate::recognise::transcriber::{TranscriberHandle, TranscriberReceiver, TranscriberSender};
use crate::recognise::Model;
use crate::sniff::Sniffer;
use crate::speak::Speaker;
use crate::{database, file, monitoring, sniff};

pub struct Interactor {
    pub listener: Listener,
    sniffer: Sniffer,
    interface: String,
    pub speaker: Speaker,
    voices: VecDeque<String>,
    pub sensitivity: f32,
    model: Model,
    data_dir: PathBuf,
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

    /// Set up a database connection and begin a new session of interactions with a list of queries.
    ///
    /// This will create a [`Listener`], a [`Sniffer`], a [`Speaker`] and use the existing [`TranscriberHandle`] for
    /// transcription.
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
    /// # use varys::recognise::{Model, Recogniser};
    /// # use varys::recognise::transcriber::Transcriber;
    /// let (_, transcriber_handle) = Transcriber::new(Recogniser::with_model(Model::default()).unwrap());
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
    ///     .start(queries, &assistant::from("Siri"), transcriber_handle)
    ///     .await
    ///     .unwrap();
    /// #     })
    /// ```
    pub async fn start<A: VoiceAssistant>(
        &mut self,
        mut queries: Vec<Query>,
        assistant: &A,
        mut transcriber_handle: TranscriberHandle<Interaction>,
    ) -> Result<(), Error> {
        let voice = self.next_voice()?;
        let (mut session, session_path, database_pool) = self.create_session(voice.clone()).await?;
        self.listener.recording_timeout = Some(assistant.recording_timeout());
        queries.shuffle(&mut rand::thread_rng());

        info!("Starting {}", session);

        for query in queries {
            if let Err(error) = monitoring::ping(&format!("Interaction started: {query}")).await {
                warn!("Failed to notify monitoring about interaction: {}", error);
            }

            match self
                .interaction(
                    &query,
                    &session,
                    &session_path,
                    &database_pool,
                    assistant.silence_after_talking(),
                )
                .await
            {
                Ok((interaction, audio)) => {
                    transcriber_handle = match transcriber_handle {
                        TranscriberHandle::Sender(sender) => sender,
                        TranscriberHandle::Receiver(receiver) => {
                            Self::complete_interaction(receiver, &database_pool).await?
                        }
                    }
                    .transcribe(interaction, audio)
                    .into();
                }
                Err(error) => {
                    error!("An interaction did not complete successfully: {error}");

                    if let Error::RecordingTimeout = error {
                        assistant.reset_assistant(self)?;
                    }
                }
            }

            assistant.stop_assistant(self)?;
        }

        // complete the last interaction and stop the transcriber
        match transcriber_handle {
            TranscriberHandle::Sender(sender) => sender,
            TranscriberHandle::Receiver(receiver) => {
                Self::complete_interaction(receiver, &database_pool).await?
            }
        }
        .stop();

        // complete the session
        session.complete(&database_pool).await?;

        Ok(())
    }

    fn next_voice(&mut self) -> Result<String, Error> {
        let voice = self.voices.pop_front().ok_or(Error::NoVoiceProvided)?;

        self.voices.push_back(voice.clone());
        self.speaker.set_voice(&voice)?;
        Ok(voice)
    }

    async fn create_session(&self, voice: String) -> Result<(Session, PathBuf, PgPool), Error> {
        let database_pool = database::connect().await?;
        let mut session = Session::create(
            &database_pool,
            &InteractorConfig {
                interface: self.interface.to_string(),
                voice,
                sensitivity: self.sensitivity.to_string(),
                model: self.model.to_string(),
            },
        )
        .await?;
        let session_path = self
            .data_dir
            .join(Path::new(&format!("sessions/session_{}", session.id)));

        fs::create_dir_all(&session_path)?;
        debug!("Storing data files at {}", session_path.to_string_lossy());
        session.data_dir = Some(session_path.to_string_lossy().to_string());
        session.update(&database_pool).await?;

        Ok((session, session_path, database_pool))
    }

    async fn interaction(
        &mut self,
        query: &Query,
        session: &Session,
        session_path: &Path,
        pool: &PgPool,
        silence_after_talking: Duration,
    ) -> Result<(Interaction, AudioData), Error> {
        info!("Starting interaction with \"{query}\"");

        // prepare the interaction
        let mut interaction = Interaction::create(pool, session, query).await?;
        let capture_path = session_path.join(capture_file_name(session, &interaction));
        let query_audio_path = session_path.join(audio_file_name(session, &interaction, "query"));
        let response_audio_path =
            session_path.join(audio_file_name(session, &interaction, "response"));

        // start the sniffer
        let sniffer_instance = self.sniffer.start(&capture_path)?;

        // begin recording the query
        let query_instance = self.listener.start()?;

        // say the query
        interaction.query_duration = Some(self.speaker.say(&query.text, true)?);

        // stop recording the query
        let query_audio = query_instance.stop()?;

        file::audio::write_audio(&query_audio_path, &query_audio)?;
        interaction.query_file = Some(file::file_name_or_full(&query_audio_path));
        interaction.update(pool).await?;

        // record the response
        let response_audio = self
            .listener
            .record_until_silent(silence_after_talking, self.sensitivity)?;

        interaction.response_duration = Some(response_audio.duration_ms());
        file::audio::write_audio(&response_audio_path, &response_audio)?;
        interaction.response_file = Some(file::file_name_or_full(&response_audio_path));
        interaction.update(pool).await?;

        // finish the sniffer
        let stats = sniffer_instance.stop()?;

        info!("{stats}");
        interaction.capture_file = Some(file::file_name_or_full(&capture_path));
        interaction.update(pool).await?;

        // at this point, the interaction is not yet complete because the response will later be
        // transcribed in a separate thread
        Ok((interaction, response_audio))
    }

    async fn complete_interaction(
        receiver: TranscriberReceiver<Interaction>,
        database_pool: &PgPool,
    ) -> Result<TranscriberSender<Interaction>, Error> {
        let (sender, interaction) = receiver.receive();
        let mut interaction = interaction?;

        info!("Transcription of {interaction} done, completing it...");

        interaction.complete(database_pool).await?;
        Ok(sender)
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
