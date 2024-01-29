use std::time::Duration;

use async_trait::async_trait;
use log::warn;

use crate::assistant::interactor::Interactor;
use crate::assistant::siri::Siri;
use crate::database::query::Query;
use crate::error::Error;

pub mod interactor;
pub mod siri;

/// This trait is implemented by all voice assistants supported by varys.
#[async_trait]
pub trait VoiceAssistant {
    /// The name of the voice assistant.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use varys::assistant::{from, VoiceAssistant};
    /// # use varys::assistant::siri::Siri;
    /// assert_eq!(Siri {}.name().as_str(), "Siri");
    /// ```
    fn name(&self) -> String;

    /// Set up voice recognition for a voice assistant.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use varys::assistant::{from, VoiceAssistant};
    /// # let assistant = from("Siri");
    /// assistant.setup().unwrap();
    /// ```
    fn setup(&self) -> Result<(), Error>;

    /// Start running interactions until all `queries` have been used up.
    ///
    /// # Arguments
    ///
    /// * `interactor`: The interactor to use.
    /// * `queries`: A list of queries to use for the interactions. Each line should contain one
    /// query.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use varys::assistant::{from, VoiceAssistant};
    /// # use varys::assistant::interactor::Interactor;
    /// # use varys::database::query::Query;
    /// # use varys::recognise::Model;
    /// let assistant = from("Siri");
    /// let mut interactor = Interactor::new(
    ///     "en0".to_string(),
    ///     vec!["Zoe".to_string()],
    ///     0.01,
    ///     Model::Large,
    ///     PathBuf::from("./data")
    /// )
    /// .unwrap();
    /// let queries = Query::read_toml(&PathBuf::from("data/test_queries.txt")).unwrap();
    /// # tokio::runtime::Builder::new_current_thread()
    /// #     .enable_all()
    /// #     .build()
    /// #     .unwrap()
    /// #     .block_on(async {
    /// assistant.interact(interactor, queries).await.unwrap();
    /// #     })
    /// ```
    async fn interact(&self, interactor: Interactor, queries: Vec<Query>) -> Result<(), Error>;

    /// Stop the current interaction with the voice assistant.
    ///
    /// # Arguments
    ///
    /// * `interactor`: The interactor to use to reset the assistant.
    fn stop_assistant(&self, interactor: &mut Interactor) -> Result<(), Error>;

    /// Reset the voice assistant to a state in which it can be used again. This is used when there are timeouts that
    /// might come from music playing or alarms ringing.
    ///
    /// # Arguments
    ///
    /// * `interactor`: The interactor to use to reset the assistant.
    fn reset_assistant(&self, interactor: &mut Interactor) -> Result<(), Error>;

    /// Test a number of voices by saying an example sentence for each one.
    ///
    /// The voices are tested in the order they are passed in.
    ///$
    /// # Arguments
    ///
    /// * `voices`: The voices to test.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use varys::assistant::{from, VoiceAssistant};
    /// # let assistant = from("Siri");
    /// let voices = vec!["Zoe".to_string(), "Isha".to_string()];
    /// assistant.test_voices(voices).unwrap();
    /// ```
    fn test_voices(&self, voices: Vec<String>) -> Result<(), Error>;

    /// The length of silence indicating that the assistant is done talking.
    fn silence_after_talking(&self) -> Duration;

    /// The amount of time to wait between interactions to make sure the voice assistant is ready.
    fn silence_between_interactions(&self) -> Duration;

    /// The maximum time to record for before cancelling an interaction.
    fn recording_timeout(&self) -> Duration;
}

/// Create a voice assistant from its name. Currently, only Siri is supported.
///
/// Pass `None` to get the default assistant.
///
/// # Arguments
///
/// * `name`: The optional name of the voice assistant.
///
/// # Examples
///
/// ```
/// # use varys::assistant::{from, VoiceAssistant};
/// assert_eq!(from("").name().as_str(), "Siri");
/// assert_eq!(from("Siri").name().as_str(), "Siri");
/// assert_eq!(from("siri").name().as_str(), "Siri");
/// ```
pub fn from(name: &str) -> impl VoiceAssistant {
    match name.to_lowercase().as_str() {
        "siri" => Siri {},
        _ => {
            warn!("Unknown voice assistant: {name}, assuming default");

            Siri {}
        }
    }
}
