use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Config(#[from] burn::config::ConfigError),
    #[error(transparent)]
    Recorder(#[from] burn::record::RecorderError),
    #[error("Cannot turn an empty list of packets into a trace")]
    EmptyTrace,
    #[error("At most {0} labels are supported")]
    TooManyLabels(usize),
    #[error("Dataset proportions must be between 0 and 1")]
    ProportionError,
    #[error("Dataset proportions do not add up to 1")]
    ProportionSumError,
    #[error("Dataset too small for the given proportions (one or more partitions would be empty)")]
    DatasetTooSmall,
    #[error("Cannot load traffic trace")]
    CannotLoadTrace,
}
