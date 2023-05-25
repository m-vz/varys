#[cfg(target_os = "macos")]
use cocoa_foundation::{base::id, foundation::NSRunLoop};
use lerp::Lerp;
use log::info;
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};
use thiserror::Error;
use tts::{Features, Tts, Voice};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Tts(#[from] tts::Error),
    #[error("Required feature {0} is unsupported")]
    UnsupportedFeature(String),
    #[error("Voice {0} is not available or does not exist")]
    VoiceNotAvailable(String),
}

/// A speaker that can synthesize voices.
pub struct Speaker {
    tts: Tts,
    available_voices: Vec<Voice>,
}

impl Speaker {
    /// Create a new speaker and load all available voices.
    pub fn new() -> Result<Self, Error> {
        let tts = Tts::default()?;

        check_features(&tts)?;

        let available_voices = tts.voices()?;
        let speaker = Speaker {
            tts,
            available_voices,
        };

        speaker.tts.on_utterance_begin(Some(Box::new(|id| {
            info!("Started saying utterance {:?}", id);
        })))?;
        speaker.tts.on_utterance_end(Some(Box::new(|id| {
            info!("Finished saying utterance {:?}", id);
        })))?;
        speaker.tts.on_utterance_stop(Some(Box::new(|id| {
            info!("Stopped saying utterance {:?}", id);
        })))?;

        Ok(speaker)
    }

    /// Set the voice that should be spoken with.
    ///
    /// Returns an error if a voice with the given name is not available on the current platform.
    ///
    /// # Examples
    ///
    /// ```
    /// use speaker::{Error, Speaker};
    /// #[cfg(target_os = "macos")]
    /// assert_eq!(Speaker::new().set_voice("Isha"), Ok(()));
    /// assert_eq!(Speaker::new().set_voice("Invalid Name"), Err(Error::VoiceNotAvailable("Invalid Name".to_string())));
    /// ```
    pub fn set_voice(&mut self, name: &str) -> Result<(), Error> {
        let voice = self.available_voices.iter().find(|v| v.name() == name);

        if let Some(voice) = voice {
            self.tts.set_voice(voice)?;

            Ok(())
        } else {
            Err(Error::VoiceNotAvailable(name.to_string()))
        }
    }

    /// Set the speaking volume.
    ///
    /// The volume is clamped between `0`, the lowest volume, and `1`, the highest volume.
    ///
    /// # Examples
    ///
    /// ```
    /// use speaker::Speaker;
    /// assert_eq!(Speaker::new().set_volume(0.8_f32), Ok(()));
    /// ```
    pub fn set_volume(&mut self, volume: f32) -> Result<(), Error> {
        let min = self.tts.min_volume();
        let max = self.tts.max_volume();
        self.tts.set_volume(min.lerp_bounded(max, volume))?;

        Ok(())
    }

    /// Reset the speaking volume to the system default.
    ///
    /// # Examples
    ///
    /// ```
    /// use speaker::Speaker;
    /// assert_eq!(Speaker::new().reset_volume(), Ok(()));
    /// ```
    pub fn reset_volume(&mut self) -> Result<(), Error> {
        self.tts.set_volume(self.tts.normal_volume())?;

        Ok(())
    }

    /// Set the speaking rate.
    ///
    /// The rate is clamped between `0`, the lowest rate, and `1`, the highest rate.
    ///
    /// # Examples
    ///
    /// ```
    /// use speaker::Speaker;
    /// assert_eq!(Speaker::new().set_rate(0.8_f32), Ok(()));
    /// ```
    pub fn set_rate(&mut self, rate: f32) -> Result<(), Error> {
        let min = self.tts.min_rate();
        let max = self.tts.max_rate();
        self.tts.set_rate(min.lerp_bounded(max, rate))?;

        Ok(())
    }

    /// Reset the speaking rate to the system default.
    ///
    /// # Examples
    ///
    /// ```
    /// use speaker::Speaker;
    /// assert_eq!(Speaker::new().reset_rate(), Ok(()));
    /// ```
    pub fn reset_rate(&mut self) -> Result<(), Error> {
        self.tts.set_rate(self.tts.normal_rate())?;

        Ok(())
    }

    /// Say a phrase in the current voice, rate and volume.
    ///
    /// Interrupts any previous speaking if `interrupt` is set.
    ///
    /// # Examples
    ///
    /// ```
    /// use speaker::Speaker;
    /// assert_eq!(Speaker::new().say("Hello world.".to_string(), false), Ok(()));
    /// ```
    pub fn say(&mut self, text: String, interrupt: bool) -> Result<(), Error> {
        self.tts.speak(text, interrupt)?;

        Ok(())
    }
}

/// On macOS, a run loop is required because speaking is non-blocking.
///
/// Call this to prevent the program from exiting before anything has been said.
pub fn start_run_loop() {
    #[cfg(target_os = "macos")]
    unsafe {
        let run_loop: id = NSRunLoop::currentRunLoop();
        let _: () = msg_send![run_loop, run];
    }
}

/// Check whether the necessary tts features are available on this platform.
fn check_features(tts: &Tts) -> Result<(), Error> {
    let Features {
        utterance_callbacks,
        rate,
        volume,
        voice,
        ..
    } = tts.supported_features();

    for (available, name) in [
        (utterance_callbacks, "utterance callbacks"),
        (rate, "speaking rate"),
        (volume, "speaking volume"),
        (voice, "voices"),
    ] {
        if !available {
            return Err(Error::UnsupportedFeature(name.to_string()));
        }
    }

    Ok(())
}
