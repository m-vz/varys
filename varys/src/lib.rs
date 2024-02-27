use clap::crate_version;

pub mod assistant;
pub mod cli;
mod dataset;
pub mod error;
pub mod monitoring;
pub mod query;

pub fn version() -> String {
    crate_version!().to_string()
}
