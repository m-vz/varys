use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    DatabaseError(#[from] varys_database::error::Error),
    #[error(transparent)]
    AudioError(#[from] varys_audio::error::Error),
    #[error(transparent)]
    NetworkError(#[from] varys_network::error::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Unable to read dotenv file: {0}")]
    Dotenv(String),
    #[error(transparent)]
    TomlDeserializeError(#[from] toml::de::Error),
    #[error("At least one voice is required")]
    NoVoiceProvided,

    // monitoring
    #[error("Connection to monitoring failed: {0}")]
    MonitoringConnectionFailed(reqwest::Error),
    #[error("Environment variable VARYS_MONITORING_URL is missing")]
    MissingMonitoringUrl,
    #[error("The monitoring url {0} is invalid")]
    InvalidMonitoringUrl(String),
}
