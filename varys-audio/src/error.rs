use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Value is out of range")]
    OutOfRange,
    #[error("Audio device not found")]
    AudioDeviceNotFound,
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
    #[error("OPUS error: {0}")]
    Opus(String),
    #[error("CPAL error: {0}")]
    Cpal(String),
    #[error("Hound error: {0}")]
    Hound(String),

    // tts
    #[error("Required feature {0} is unsupported")]
    UnsupportedFeature(String),
    #[error("Voice {0} is not available or does not exist")]
    VoiceNotAvailable(String),
    #[error("Tts error: {0}")]
    Tts(String),

    // stt
    #[error("Recording is too short to be processed by whisper")]
    RecordingTooShort,
    #[error("The transcriber has stopped")]
    TranscriberStopped,
    #[error("Failed to create new whisper context")]
    WhisperContext,
    #[error("An error occurred during recognition")]
    Recognition,
    #[error("Whisper error: {0}")]
    Whisper(String),
}

#[cfg(target_os = "macos")]
impl From<tts::Error> for Error {
    fn from(value: tts::Error) -> Self {
        match value {
            tts::Error::Io(err) => err.into(),
            tts::Error::UnsupportedFeature => Error::UnsupportedFeature(String::new()),
            tts::Error::OutOfRange => Error::OutOfRange,
            _ => Error::Tts(value.to_string()),
        }
    }
}

impl From<cpal::BuildStreamError> for Error {
    fn from(value: cpal::BuildStreamError) -> Self {
        match value {
            cpal::BuildStreamError::DeviceNotAvailable => Error::AudioDeviceNotFound,
            cpal::BuildStreamError::StreamConfigNotSupported => Error::ConfigurationNotSupported,
            _ => Error::Cpal(value.to_string()),
        }
    }
}

impl From<cpal::SupportedStreamConfigsError> for Error {
    fn from(value: cpal::SupportedStreamConfigsError) -> Self {
        match value {
            cpal::SupportedStreamConfigsError::DeviceNotAvailable => Error::AudioDeviceNotFound,
            _ => Error::Cpal(value.to_string()),
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
            _ => Error::Opus(value.to_string()),
        }
    }
}

impl From<cpal::PlayStreamError> for Error {
    fn from(value: cpal::PlayStreamError) -> Self {
        match value {
            cpal::PlayStreamError::DeviceNotAvailable => Error::AudioDeviceNotFound,
            _ => Error::Cpal(value.to_string()),
        }
    }
}

impl From<hound::Error> for Error {
    fn from(value: hound::Error) -> Self {
        match value {
            hound::Error::IoError(err) => err.into(),
            _ => Error::Hound(value.to_string()),
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
            _ => Error::Whisper(value.to_string()),
        }
    }
}
