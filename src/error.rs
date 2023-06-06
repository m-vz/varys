use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    // speak
    #[error(transparent)]
    Tts(#[from] tts::Error),
    #[error("Required feature {0} is unsupported")]
    UnsupportedFeature(String),
    #[error("Voice {0} is not available or does not exist")]
    VoiceNotAvailable(String),

    // listen
    /// Error that happens if no audio input device was found.
    #[error("Audio input device not found")]
    MissingInputDevice,
    /// Error that happens if the audio input device does not support a required configuration.
    #[error("Audio device does not support required configuration")]
    ConfigurationNotSupported,
    /// Error that happens when trying to access audio data while it is still being recorded.
    #[error("Recording still running")]
    StillRecording,
    #[error(transparent)]
    BuildStream(#[from] cpal::BuildStreamError),
    #[error(transparent)]
    PlayStream(#[from] cpal::PlayStreamError),
    #[error(transparent)]
    RecordingFailed(#[from] std::sync::PoisonError<Vec<f32>>),

    // audio
    #[error(
        "Downsampling requires the target sample rate to be a divisor of the current sample rate."
    )]
    NoDivisor,
    #[error(transparent)]
    Hound(#[from] hound::Error),

    // recognise
    #[error(transparent)]
    WhisperError(#[from] whisper_rs::WhisperError),

    // sniff
    #[error("No default network device was found.")]
    DefaultDeviceNotFound,
    #[error("Could not find device {0}.")]
    DeviceNotFound(String),
    #[error("Tried to stop sniffer that was not running.")]
    CannotStop,
    #[error("Did not receive sniffer stats.")]
    NoStatsReceived,
    #[error(transparent)]
    Pcap(#[from] pcap::Error),

    // interact
    #[error(transparent)]
    InputOutput(#[from] std::io::Error),
}
