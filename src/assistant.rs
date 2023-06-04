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

pub trait Assistant: Setup {}
impl<T: Setup> Assistant for T {}

pub trait Setup {
    fn setup(&self) -> Result<(), Error>;
}

pub fn from(name: Option<String>) -> impl Assistant {
    match name.unwrap_or(String::new()).to_lowercase().as_str() {
        "siri" => Siri {},
        _ => Siri {},
    }
}
