use std::sync::mpsc::{channel, TryRecvError};

#[cfg(target_os = "macos")]
use cocoa_foundation::{
    base::id,
    foundation::{NSDefaultRunLoopMode, NSRunLoop},
};
use lerp::Lerp;
use log::debug;
#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};
use tts::{Features, Tts, Voice};

use crate::error::Error;

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

        Ok(speaker)
    }

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
    /// # use varys::error::Error;
    /// # use varys::speak::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    ///
    /// assert!(speaker.set_voice("Jamie").is_ok());
    /// assert!(speaker.set_voice("com.apple.voice.premium.en-GB.Malcolm").is_ok());
    /// # }
    /// ```
    ///
    /// ```
    /// # use varys::error::Error;
    /// # use varys::speak::Speaker;
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
        let voice = self
            .available_voices
            .iter()
            .find(|v| v.id() == id_or_name || v.name() == id_or_name);

        if let Some(voice) = voice {
            self.tts.set_voice(voice)?;

            Ok(())
        } else {
            Err(Error::VoiceNotAvailable(id_or_name.to_string()))
        }
    }

    /// Set the speaking volume.
    ///
    /// The volume is clamped between `0`, the lowest volume, and `1`, the highest volume.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys::speak::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    /// speaker.set_volume(0.8_f32).unwrap();
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
    /// # use varys::speak::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    /// speaker.reset_volume().unwrap();
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
    /// # use varys::speak::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    /// speaker.set_rate(0.8_f32).unwrap();
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
    /// # use varys::speak::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    /// speaker.reset_rate().unwrap();
    /// ```
    pub fn reset_rate(&mut self) -> Result<(), Error> {
        self.tts.set_rate(self.tts.normal_rate())?;

        Ok(())
    }

    /// Say a phrase in the current voice, rate and volume.
    ///
    /// Interrupts any previous speaking if `interrupt` is set.
    ///
    /// This blocks the current thread until speaking has finished.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys::speak::Speaker;
    /// let mut speaker = Speaker::new().unwrap();
    /// speaker.say("", false).unwrap();
    /// ```
    pub fn say(&mut self, text: &str, interrupt: bool) -> Result<(), Error> {
        debug!("Saying \"{}\"", text);

        let (sender, receiver) = channel();
        self.tts.on_utterance_end(Some(Box::new(move |_| {
            let _ = sender.send(());
        })))?;

        self.tts.speak(text, interrupt)?;

        #[cfg(target_os = "macos")]
        {
            unsafe {
                let run_loop: id = NSRunLoop::currentRunLoop();
                let date: id = msg_send![class!(NSDate), distantFuture];
                while receiver.try_recv() == Err(TryRecvError::Empty) {
                    let _: () = msg_send![run_loop, runMode:NSDefaultRunLoopMode beforeDate:date];
                }
            }
        }

        Ok(())
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
