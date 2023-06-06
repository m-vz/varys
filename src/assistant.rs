pub mod siri;

use crate::assistant::siri::Siri;
use crate::{cli, speak};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    CliIo(#[from] cli::interact::Error),
    #[error(transparent)]
    Speaker(#[from] speak::Error),
}

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
    /// # let assistant = from(None);
    /// assistant.setup().unwrap();
    /// ```
    fn setup(&self) -> Result<(), Error>;

    /// Start running interactions until all `queries` have been used up.
    ///
    /// # Arguments
    ///
    /// * `interface`: The network interface to capture traffic from.
    /// * `voice`: The system voice to use for the interactions.
    /// * `queries`: A list of queries to use for the interactions. Each line should contain one query.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use varys::assistant::{from, VoiceAssistant};
    /// # let assistant = from(None);
    /// assistant.interact("ap1", "Zoe", PathBuf::from("data/test_queries.txt".to_string())).unwrap();
    /// ```
    fn interact(&self, interface: &str, voice: &str, queries: PathBuf) -> Result<(), crate::Error>;

    /// Test a number of voices by saying an example sentence for each one.
    ///
    /// The voices are tested in the order they are passed in.
    ///
    /// # Arguments
    ///
    /// * `voices`: The voices to test.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use varys::assistant::{from, VoiceAssistant};
    /// # let assistant = from(None);
    /// let voices = vec!["Zoe".to_string(), "Isha".to_string()];
    /// assistant.test_voices(voices).unwrap();
    /// ```
    fn test_voices(&self, voices: Vec<String>) -> Result<(), Error>;
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
/// assert_eq!(from(None).name().as_str(), "Siri");
/// assert_eq!(from(Some("Siri")).name().as_str(), "Siri");
/// assert_eq!(from(Some("siri")).name().as_str(), "Siri");
/// ```
pub fn from(name: Option<&str>) -> impl VoiceAssistant {
    match name.unwrap_or("").to_lowercase().as_str() {
        "siri" => Siri {},
        _ => Siri {},
    }
}
