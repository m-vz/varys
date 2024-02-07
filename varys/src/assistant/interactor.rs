use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::crate_version;
use log::{debug, error, info, warn};
use rand::prelude::SliceRandom;

use chrono::Utc;
use varys_audio::audio::AudioData;
use varys_audio::listen::Listener;
use varys_audio::stt::transcribe::Transcribe;
use varys_audio::stt::transcriber::{TranscriberHandle, TranscriberReceiver, TranscriberSender};
use varys_audio::stt::Model;
use varys_audio::tts::Speaker;
use varys_database::connection::DatabaseConnection;
use varys_database::database;
use varys_database::database::interaction::Interaction;
use varys_database::database::interactor_config::InteractorConfig;
use varys_database::database::session::Session;
use varys_network::sniff;
use varys_network::sniff::Sniffer;

use crate::assistant::VoiceAssistant;
use crate::error::Error;
use crate::monitoring;
use crate::query::Query;

pub struct TranscribeInteraction(Interaction);

impl Transcribe for TranscribeInteraction {
    fn transcribed(&mut self, text: String) {
        self.0.response = Some(text);
    }
}

impl From<Interaction> for TranscribeInteraction {
    fn from(interaction: Interaction) -> Self {
        Self(interaction)
    }
}

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
    /// # use varys_audio::stt::Model;
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
    /// # use std::ops::Deref;
    /// use std::path::PathBuf;
    /// # use varys::assistant;
    /// # use varys::assistant::interactor::Interactor;
    /// # use varys::query::Query;
    /// # use varys_audio::stt::{Model, Recogniser};
    /// # use varys_audio::stt::transcriber::Transcriber;
    /// let (_, transcriber_handle) = Transcriber::new(Recogniser::with_model(Model::default()).unwrap());
    /// let mut interactor = Interactor::new(
    ///     "en0".to_string(),
    ///     vec!["Ava".to_string()],
    ///     0.01,
    ///     Model::Large,
    ///     PathBuf::from("./data")
    /// )
    /// .unwrap();
    /// let mut queries = vec![
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
    ///     .start(&mut queries, assistant::from("Siri").as_ref(), transcriber_handle)
    ///     .await
    ///     .unwrap();
    /// #     })
    /// ```
    pub async fn start(
        &mut self,
        queries: &mut Vec<Query>,
        assistant: &dyn VoiceAssistant,
        mut transcriber_handle: TranscriberHandle<TranscribeInteraction>,
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
                    query,
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
                    .transcribe(interaction.into(), audio)
                    .into();
                }
                Err(error) => {
                    error!("An interaction did not complete successfully: {error}");

                    if let Error::AudioError(varys_audio::error::Error::RecordingTimeout) = error {
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

    async fn create_session(
        &self,
        voice: String,
    ) -> Result<(Session, PathBuf, DatabaseConnection), Error> {
        let database_connection = database::connect().await?;
        let mut session = Session::create(
            &database_connection,
            &InteractorConfig {
                interface: self.interface.to_string(),
                voice,
                sensitivity: self.sensitivity.to_string(),
                model: self.model.to_string(),
            },
            crate_version!().to_string(),
        )
        .await?;
        let session_path = self
            .data_dir
            .join(Path::new(&format!("sessions/session_{}", session.id)));

        fs::create_dir_all(&session_path)?;
        debug!("Storing data files at {}", session_path.to_string_lossy());
        session.data_dir = Some(session_path.to_string_lossy().to_string());
        session.update(&database_connection).await?;

        Ok((session, session_path, database_connection))
    }

    async fn interaction(
        &mut self,
        query: &Query,
        session: &Session,
        session_path: &Path,
        connection: &DatabaseConnection,
        silence_after_talking: Duration,
    ) -> Result<(Interaction, AudioData), Error> {
        info!("Starting interaction with \"{query}\"");

        // prepare the interaction
        let mut interaction =
            Interaction::create(connection, session, &query.text, &query.category).await?;
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

        varys_audio::file::write_audio(&query_audio_path, &query_audio)?;
        interaction.query_file = Some(file_name_or_full(&query_audio_path));
        interaction.update(connection).await?;

        // record the response
        let response_audio = self
            .listener
            .record_until_silent(silence_after_talking, self.sensitivity)?;

        interaction.response_duration = Some(response_audio.duration_ms());
        varys_audio::file::write_audio(&response_audio_path, &response_audio)?;
        interaction.response_file = Some(file_name_or_full(&response_audio_path));
        interaction.update(connection).await?;

        // finish the sniffer
        let stats = sniffer_instance.stop()?;

        info!("{stats}");
        interaction.capture_file = Some(file_name_or_full(&capture_path));
        interaction.update(connection).await?;

        // at this point, the interaction is not yet complete because the response will later be
        // transcribed in a separate thread
        Ok((interaction, response_audio))
    }

    async fn complete_interaction(
        receiver: TranscriberReceiver<TranscribeInteraction>,
        database_connection: &DatabaseConnection,
    ) -> Result<TranscriberSender<TranscribeInteraction>, Error> {
        let (sender, interaction) = receiver.receive();
        let mut interaction = interaction?;

        info!("Transcription of {} done, completing it...", interaction.0);

        interaction.0.complete(database_connection).await?;
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

/// Returns the file name if it exists. Otherwise, returns the full path.
///
/// # Arguments
///
/// * `file_path`: The path to the file to get the name from.
///
/// # Examples
///
/// ```
/// # use std::path::Path;
/// # use varys::assistant::interactor::file_name_or_full;
/// assert_eq!(file_name_or_full(Path::new("path/to/text.txt")), "text.txt");
/// ```
pub fn file_name_or_full(file_path: &Path) -> String {
    file_path
        .file_name()
        .unwrap_or(file_path.as_os_str())
        .to_string_lossy()
        .to_string()
}
