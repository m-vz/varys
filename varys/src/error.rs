use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Value is out of range")]
    OutOfRange,
    #[error("Unable to read dotenv file: {0}")]
    Dotenv(String),
    #[error("Could not serialize toml: {0}")]
    TomlSerializeFailed(toml::ser::Error),
    #[error("Could not deserialize toml: {0}")]
    TomlDeserializeFailed(toml::de::Error),

    // tts
    #[error("Required feature {0} is unsupported")]
    UnsupportedFeature(String),
    #[error("Voice {0} is not available or does not exist")]
    VoiceNotAvailable(String),
    #[error("At least one voice is required")]
    NoVoiceProvided,
    #[error("Tts error")]
    Tts,

    // audio
    #[error("Audio input device not found")]
    AudioInputDeviceNotFound,
    #[error("Audio device does not support required configuration")]
    ConfigurationNotSupported,
    #[error("Tried to access audio data while recording still running")]
    StillRecording,
    #[error("Could not access recorded audio data")]
    RecordingFailed,
    #[error("Encountered timeout while recording audio")]
    RecordingTimeout,
    #[error(
        "Downsampling requires the target sample rate to be a divisor of the current sample rate"
    )]
    NoDivisor,
    #[error(
        "Opus does not support sample rate {0}hz. Use one of 8000, 12000, 16000, 24000 or 48000"
    )]
    UnsupportedSampleRate(u32),
    #[error("Opus does not support more than two channels (got audio data with {0} channels)")]
    UnsupportedChannelCount(u16),
    #[error("OPUS error")]
    Opus,
    #[error("CPAL error")]
    Cpal,
    #[error("Hound error")]
    Hound,

    // stt
    #[error("Recording is too short to be processed by whisper")]
    RecordingTooShort,
    #[error("The transcriber has stopped")]
    TranscriberStopped,
    #[error("Failed to create new whisper context")]
    WhisperContext,
    #[error("An error occurred during recognition")]
    Recognition,
    #[error("Whisper error")]
    Whisper,

    // network
    #[error("No default network device was found")]
    DefaultDeviceNotFound,
    #[error("Could not find device {0}")]
    NetworkDeviceNotFound(String),
    #[error("Tried to stop sniffer that was not running")]
    CannotStop,
    #[error("Did not receive sniffer stats")]
    NoStatsReceived,
    #[error("Pcap error")]
    Pcap,

    // database
    #[error("Database error: {0}")]
    Database(String),
    #[error("Database migration error: {0}")]
    DatabaseMigration(String),
    #[error("Environment variable DATABASE_URL is missing")]
    MissingDatabaseUrl,

    // monitoring
    #[error("Connection to monitoring failed: {0}")]
    MonitoringConnectionFailed(reqwest::Error),
    #[error("Environment variable VARYS_MONITORING_URL is missing")]
    MissingMonitoringUrl,
    #[error("The monitoring url {0} is invalid")]
    InvalidMonitoringUrl(String),
}

impl From<tts::Error> for Error {
    fn from(value: tts::Error) -> Self {
        match value {
            tts::Error::Io(err) => err.into(),
            tts::Error::UnsupportedFeature => Error::UnsupportedFeature(String::new()),
            tts::Error::OutOfRange => Error::OutOfRange,
            _ => Error::Tts,
        }
    }
}

impl From<cpal::BuildStreamError> for Error {
    fn from(value: cpal::BuildStreamError) -> Self {
        match value {
            cpal::BuildStreamError::DeviceNotAvailable => Error::AudioInputDeviceNotFound,
            cpal::BuildStreamError::StreamConfigNotSupported => Error::ConfigurationNotSupported,
            _ => Error::Cpal,
        }
    }
}

impl From<cpal::SupportedStreamConfigsError> for Error {
    fn from(value: cpal::SupportedStreamConfigsError) -> Self {
        match value {
            cpal::SupportedStreamConfigsError::DeviceNotAvailable => {
                Error::AudioInputDeviceNotFound
            }
            _ => Error::Cpal,
        }
    }
}

impl From<audiopus::Error> for Error {
    fn from(value: audiopus::Error) -> Self {
        match value {
            audiopus::Error::InvalidSampleRate(channels) => {
                Error::UnsupportedSampleRate(channels as u32)
            }
            audiopus::Error::InvalidChannels(channels) => {
                Error::UnsupportedChannelCount(channels as u16)
            }
            _ => Error::Opus,
        }
    }
}

impl From<cpal::PlayStreamError> for Error {
    fn from(value: cpal::PlayStreamError) -> Self {
        match value {
            cpal::PlayStreamError::DeviceNotAvailable => Error::AudioInputDeviceNotFound,
            _ => Error::Cpal,
        }
    }
}

impl From<hound::Error> for Error {
    fn from(value: hound::Error) -> Self {
        match value {
            hound::Error::IoError(err) => err.into(),
            _ => Error::Hound,
        }
    }
}

impl From<whisper_rs::WhisperError> for Error {
    fn from(value: whisper_rs::WhisperError) -> Self {
        match value {
            whisper_rs::WhisperError::InitError => Error::WhisperContext,
            whisper_rs::WhisperError::UnableToCalculateSpectrogram
            | whisper_rs::WhisperError::FailedToEncode
            | whisper_rs::WhisperError::FailedToDecode => Error::Recognition,
            _ => Error::Whisper,
        }
    }
}

impl From<pcap::Error> for Error {
    fn from(value: pcap::Error) -> Self {
        match value {
            pcap::Error::IoError(err) => std::io::Error::from(err).into(),
            _ => Error::Pcap,
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Error::Database(value.to_string())
    }
}

impl From<sqlx::migrate::MigrateError> for Error {
    fn from(value: sqlx::migrate::MigrateError) -> Self {
        Error::DatabaseMigration(value.to_string())
    }
}

impl From<toml::ser::Error> for Error {
    fn from(value: toml::ser::Error) -> Self {
        Error::TomlSerializeFailed(value)
    }
}

impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Error::TomlDeserializeFailed(value)
    }
}
