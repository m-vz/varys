pub mod audio;

use crate::listen::audio::AudioData;
use crate::recognise::Recogniser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BuildStreamError, Device, PlayStreamError, SampleFormat, StreamConfig};
use log::{debug, error, info};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
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
    BuildStream(#[from] BuildStreamError),
    #[error(transparent)]
    PlayStream(#[from] PlayStreamError),
    #[error(transparent)]
    RecordingFailed(#[from] PoisonError<Vec<f32>>),
}

/// A listener that can parse voice input.
pub struct Listener {
    device: Device,
    device_config: StreamConfig,
    /// The optional maximum duration to record for.
    /// Use this to stop any recording longer than the specified duration.
    /// This ensures the listener does not record forever if there is interference or noise.
    ///
    /// Defaults to [`Listener::DEFAULT_RECORDING_TIMEOUT`].
    pub recording_timeout: Option<Duration>,
}

impl Listener {
    const DEFAULT_RECORDING_TIMEOUT: Option<Duration> = Some(Duration::from_secs(60));

    /// Create a new listener using the system default input device.
    ///
    /// Returns an error if no input device was found or if it doesn't support the required sample
    /// rate and format.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys::listen::Listener;
    /// let listener = Listener::new().unwrap();
    /// ```
    pub fn new() -> Result<Self, Error> {
        let device = cpal::default_host()
            .default_input_device()
            .ok_or(Error::MissingInputDevice)?;
        if let Ok(name) = device.name() {
            debug!("Using audio device {}", name);
        }

        let device_config: StreamConfig = device
            .supported_input_configs()
            .map_err(|_| Error::ConfigurationNotSupported)?
            .find(|config| {
                config.sample_format() == SampleFormat::F32
                    && config.max_sample_rate().0 % Recogniser::SAMPLE_RATE == 0
            })
            .ok_or(Error::ConfigurationNotSupported)?
            .with_max_sample_rate()
            .into();
        debug!("Using audio input config {:?}", device_config);

        Ok(Listener {
            device,
            device_config,
            recording_timeout: Listener::DEFAULT_RECORDING_TIMEOUT,
        })
    }

    /// Record audio data.
    ///
    /// Returns an error if the audio stream could not be built or played. This might happen if the
    /// device is no longer available.
    ///
    /// # Arguments
    ///
    /// * `seconds`: How long to record for.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys::listen::Listener;
    /// let listener = Listener::new().unwrap();
    /// listener.record(0).unwrap();
    /// ```
    pub fn record(&self, seconds: u32) -> Result<AudioData, Error> {
        let writer = Arc::new(Mutex::new(Vec::with_capacity(
            (self.device_config.sample_rate.0 * seconds) as usize,
        )));
        let writer_2 = writer.clone();
        let stream = self.device.build_input_stream(
            &self.device_config,
            move |data: &[f32], _| {
                if let Ok(mut guard) = writer_2.try_lock() {
                    for &sample in data.iter() {
                        guard.push(sample);
                    }
                }
            },
            move |err| error!("Audio stream error: {}", err),
            self.recording_timeout,
        )?;

        info!("Starting recording...");
        stream.play()?;
        for second in (1..=seconds).rev() {
            info!("{}...", second);
            thread::sleep(Duration::from_secs(1));
        }
        drop(stream);
        info!("Recording done");

        let data = Arc::try_unwrap(writer)
            .map_err(|_| Error::StillRecording)?
            .into_inner()?;

        Ok(AudioData {
            data,
            channels: self.device_config.channels,
            sample_rate: self.device_config.sample_rate.0,
        })
    }
}