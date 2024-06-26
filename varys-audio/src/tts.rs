use std::time::Instant;

#[cfg(target_os = "macos")]
use cocoa_foundation::{
    base::id,
    foundation::{NSDefaultRunLoopMode, NSRunLoop},
};
#[cfg(target_os = "macos")]
use lerp::Lerp;
#[cfg(target_os = "macos")]
use log::debug;
use log::{info, trace};
#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};
#[cfg(target_os = "macos")]
use std::sync::mpsc::{channel, TryRecvError};
#[cfg(target_os = "macos")]
use tts::{Features, Tts, Voice};

use crate::error::Error;

/// A speaker that can synthesize voices.
#[cfg(target_os = "macos")]
pub struct Speaker {
    tts: Tts,
    available_voices: Vec<Voice>,
}
#[cfg(not(target_os = "macos"))]
pub struct Speaker {}

impl Speaker {
    /// Create a new speaker and load all available voices.
    pub fn new() -> Result<Self, Error> {
        #[cfg(target_os = "macos")]
        {
            let tts = Tts::default()?;

            check_features(&tts)?;

            let available_voices = tts.voices()?;
            let speaker = Speaker {
                tts,
                available_voices,
            };

            debug!(
                "Available voices: {}",
                speaker
                    .available_voices
                    .iter()
                    .map(|voice| voice.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            Ok(speaker)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(Self {})
        }
    }

    /// Create a new speaker and set the voice that should be spoken with.
    ///
    /// # Arguments
    ///
    /// * `id_or_name`: The id or name of the voice to use.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(target_os = "macos")]
    /// # {
    /// # use varys_audio::tts::Speaker;
    /// assert!(Speaker::with_voice("Ava").is_ok());
    /// # }
    /// ```
    ///
    /// ```
    /// # use varys_audio::error::Error;
    /// # use varys_audio::tts::Speaker;
    /// let invalid_speaker = Speaker::with_voice("Invalid Name");
    ///
    /// if let Err(Error::VoiceNotAvailable(text)) = invalid_speaker {
    ///     assert_eq!(text, "Invalid Name");
    /// } else {
    ///     panic!("Must return `Error::VoiceNotAvailable`");
    /// }
    /// ```
    pub fn with_voice(id_or_name: &str) -> Result<Self, Error> {
        let mut speaker = Self::new()?;

        speaker.set_voice(id_or_name)?;

        Ok(speaker)
    }

    /// Set the voice that should be spoken with.
    ///
    /// Returns an error if a voice with the given id or name is not available on the current platform.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(target_os = "macos")]
    /// # {
    /// # use varys_audio::tts::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    ///
    /// assert!(speaker.set_voice("Ava").is_ok());
    /// # }
    /// ```
    ///
    /// ```
    /// # use varys_audio::error::Error;
    /// # use varys_audio::tts::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    /// let invalid = speaker.set_voice("Invalid Name");
    ///
    /// if let Err(Error::VoiceNotAvailable(text)) = invalid {
    ///     assert_eq!(text, "Invalid Name");
    /// } else {
    ///     panic!("Must return `Error::VoiceNotAvailable`");
    /// }
    /// ```
    pub fn set_voice(&mut self, id_or_name: &str) -> Result<(), Error> {
        #[cfg(target_os = "macos")]
        {
            let voice = self
                .available_voices
                .iter()
                .find(|v| v.id() == id_or_name || v.name() == id_or_name);

            if let Some(voice) = voice {
                self.tts.set_voice(voice)?;

                info!("Using voice {}", id_or_name);

                Ok(())
            } else {
                Err(Error::VoiceNotAvailable(id_or_name.to_string()))
            }
        }
        #[cfg(not(target_os = "macos"))]
        Ok(())
    }

    /// Set the speaking volume.
    ///
    /// The volume is clamped between `0`, the lowest volume, and `1`, the highest volume.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys_audio::tts::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    /// speaker.set_volume(0.8_f32).unwrap();
    /// ```
    pub fn set_volume(&mut self, volume: f32) -> Result<(), Error> {
        #[cfg(target_os = "macos")]
        {
            let min = self.tts.min_volume();
            let max = self.tts.max_volume();
            self.tts.set_volume(min.lerp_bounded(max, volume))?;
        }

        info!("Volume set to {:.2}", volume);

        Ok(())
    }

    /// Reset the speaking volume to the system default.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys_audio::tts::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    /// speaker.reset_volume().unwrap();
    /// ```
    pub fn reset_volume(&mut self) -> Result<(), Error> {
        #[cfg(target_os = "macos")]
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
    /// # use varys_audio::tts::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    /// speaker.set_rate(0.8_f32).unwrap();
    /// ```
    pub fn set_rate(&mut self, rate: f32) -> Result<(), Error> {
        #[cfg(target_os = "macos")]
        {
            let min = self.tts.min_rate();
            let max = self.tts.max_rate();
            self.tts.set_rate(min.lerp_bounded(max, rate))?;
        }

        info!("Speaking rate set to {:.2}", rate);

        Ok(())
    }

    /// Reset the speaking rate to the system default.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys_audio::tts::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    /// speaker.reset_rate().unwrap();
    /// ```
    pub fn reset_rate(&mut self) -> Result<(), Error> {
        #[cfg(target_os = "macos")]
        self.tts.set_rate(self.tts.normal_rate())?;

        Ok(())
    }

    /// Say a phrase in the current voice, rate and volume. Returns the time in milliseconds it took
    /// to say the phrase.
    ///
    /// Interrupts any previous speaking if `interrupt` is set.
    ///
    /// This blocks the current thread until speaking has finished.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys_audio::tts::Speaker;
    /// let speaker = Speaker::new().unwrap();
    /// let speaking_duration = speaker.say("", false).unwrap();
    /// ```
    pub fn say(&self, text: &str, interrupt: bool) -> Result<i32, Error> {
        info!("Saying \"{text}\"");

        let start = Instant::now();
        #[cfg(target_os = "macos")]
        {
            let (sender, receiver) = channel();
            self.tts.on_utterance_end(Some(Box::new(move |_| {
                let _ = sender.send(());
            })))?;

            self.tts.clone().speak(text, interrupt)?;

            unsafe {
                let run_loop: id = NSRunLoop::currentRunLoop();
                let date: id = msg_send![class!(NSDate), distantFuture];
                while receiver.try_recv() == Err(TryRecvError::Empty) {
                    let _: () = msg_send![run_loop, runMode:NSDefaultRunLoopMode beforeDate:date];
                }
            }
        }
        let duration = start.elapsed().as_millis() as i32;
        trace!("Spoke for {duration}ms");

        Ok(duration)
    }
}

/// Check whether the necessary tts features are available on this platform.
#[cfg(target_os = "macos")]
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
