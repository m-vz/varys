pub mod siri;

use crate::assistant::siri::Siri;
use crate::{cli, speak};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    CliIo(#[from] cli::interact::Error),
    #[error(transparent)]
    Speaker(#[from] speak::Error),
}

pub enum Assistant {
    Siri(Siri),
    Unavailable,
}

impl From<&str> for Assistant {
    fn from(value: &str) -> Self {
        match &value.to_lowercase()[..] {
            "siri" => Assistant::Siri(Siri {}),
            _ => Assistant::Unavailable,
        }
    }
}

impl Default for Assistant {
    fn default() -> Self {
        Assistant::Siri(Siri {})
    }
}

pub trait VoiceAssistant {
    fn setup(&self) -> Result<(), Error>;
}
