use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    DatabaseError(#[from] varys_database::error::Error),
    #[error(transparent)]
    AudioError(#[from] varys_audio::error::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Unable to read dotenv file: {0}")]
    Dotenv(String),
    #[error(transparent)]
    TomlDeserializeError(#[from] toml::de::Error),
    #[error("At least one voice is required")]
    NoVoiceProvided,

    // network
    #[error("No default network device was found")]
    DefaultDeviceNotFound,
    #[error("Could not find device {0}")]
    NetworkDeviceNotFound(String),
    #[error("Tried to stop sniffer that was not running")]
    CannotStop,
    #[error("Did not receive sniffer stats")]
    NoStatsReceived,
    #[error("Pcap error: {0}")]
    Pcap(String),

    // monitoring
    #[error("Connection to monitoring failed: {0}")]
    MonitoringConnectionFailed(reqwest::Error),
    #[error("Environment variable VARYS_MONITORING_URL is missing")]
    MissingMonitoringUrl,
    #[error("The monitoring url {0} is invalid")]
    InvalidMonitoringUrl(String),
}

impl From<pcap::Error> for Error {
    fn from(value: pcap::Error) -> Self {
        match value {
            pcap::Error::IoError(err) => std::io::Error::from(err).into(),
            _ => Error::Pcap(value.to_string()),
        }
    }
}
