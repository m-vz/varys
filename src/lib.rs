pub mod assistant;
pub mod cli;
pub mod listen;
pub mod recognise;
pub mod sniff;
pub mod speak;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Speaker(#[from] speak::Error),
    #[error(transparent)]
    Listener(#[from] listen::Error),
    #[error(transparent)]
    Audio(#[from] listen::audio::Error),
    #[error(transparent)]
    Recogniser(#[from] recognise::Error),
    #[error(transparent)]
    Sniffer(#[from] sniff::Error),
    #[error(transparent)]
    Assistant(#[from] assistant::Error),
    #[error(transparent)]
    Interact(#[from] cli::interact::Error),
}
