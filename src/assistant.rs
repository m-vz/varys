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

pub trait Assistant: Setup + Interact + Test {}
impl<T: Setup + Interact + Test> Assistant for T {}

pub trait Setup {
    fn setup(&self) -> Result<(), Error>;
}

pub trait Interact {
    fn interact(&self, interface: &str, voice: &str, queries: PathBuf) -> Result<(), crate::Error>;
}

pub trait Test {
    fn test(&self, voices: Vec<String>) -> Result<(), Error>;
}

pub fn from(name: Option<&str>) -> impl Assistant {
    match name.unwrap_or("").to_lowercase().as_str() {
        "siri" => Siri {},
        _ => Siri {},
    }
}
