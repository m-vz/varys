use std::path::Path;

use async_trait::async_trait;

use crate::assistant::interactor::Interactor;
use crate::assistant::siri::Siri;
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
    /// # use varys::assistant::interactor::InteractorBuilder;
    /// # use varys::recognise::Model;
    /// let assistant = from("Siri");
    /// let mut interactor = InteractorBuilder::new(
    ///     "ap1".to_string(),
    ///     "Zoe".to_string(),
    ///     0.01,
    ///     Model::Large,
    ///     PathBuf::from("./data")
    /// )
    /// .build()
    /// .unwrap();
    /// # tokio::runtime::Builder::new_current_thread()
    /// #     .enable_all()
    /// #     .build()
    /// #     .unwrap()
    /// #     .block_on(async {
    /// assistant.interact(interactor, &PathBuf::from("data/test_queries.txt")).await.unwrap();
    /// #     })
    /// ```
    async fn interact(&self, interactor: Interactor, queries: &Path) -> Result<(), Error>;

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
        _ => Siri {},
    }
}

/// Filter comments from a list of queries.
///
/// # Arguments
///
/// * `queries`: The queries to filter.
///
/// # Examples
///
/// ```
/// # use varys::assistant::prepare_queries;
/// let unfiltered = vec!["one", "// two", "three"];
/// assert_eq!(prepare_queries(unfiltered, |q| format!("-{}", q)), vec!["-one".to_string(), "-three".to_string()]);
/// ```
pub fn prepare_queries(queries: Vec<&str>, format: fn(String) -> String) -> Vec<String> {
    queries
        .into_iter()
        .filter(|q| !q.starts_with("//"))
        .map(|q| format(q.to_string()))
        .collect()
}
