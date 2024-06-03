use std::time::Duration;

use colored::Colorize;
use log::info;

use varys_audio::tts::Speaker;

use crate::assistant::interactor::Interactor;
use crate::assistant::{Error, VoiceAssistant};
use crate::cli::{interact, key_type::KeyType};
use crate::query::Query;

/// The [`VoiceAssistant`] implementation for Alexa. Tested with the Echo Dot.
pub struct Alexa {}

impl Alexa {
    pub const PREMIUM_VOICES: &'static [&'static str] =
        &["Ava", "Karen", "Jamie", "Matilda", "Serena", "Zoe"];
}

impl VoiceAssistant for Alexa {
    fn name(&self) -> String {
        "Alexa".to_string()
    }

    fn setup(&self) -> Result<(), Error> {
        info!("Starting Alexa setup...");

        let mut speaker = Speaker::new()?;

        let voice = interact::user_input(
            &format!(
                "Choose the voice to set up (The highest quality voices on macOS are {}):",
                Alexa::PREMIUM_VOICES.join(", ")
            ),
            |i| speaker.set_voice(i).is_ok(),
            "Voice not found, enter a voice that can be used on this system:",
        )?;
        println!("Setting up Alexa for {}", voice);
        interact::user_confirmation(
            "This requires a number of sentences to be said. To continue, press",
        )?;
        interact::user_confirmation(
            "Make sure your phone is close enough to this computer to hear it.",
        )?;
        interact::user_confirmation(&format!(
            "In your Alexa App, go to {} > {} and choose {} (you might have to delete the old ID first)",
            "Settings".bright_blue(),
            "Your Profile & Family".bright_blue(),
            "Voice ID".bright_blue(),
        ))?;
        interact::user_confirmation(&format!(
            "The sentences will now be said. Press {} on your device and then",
            "Consent".bright_blue()
        ))?;
        for sentence in [
            "Alexa. What's the temperature outside?",
            "Alexa. Play music.",
            "Alexa. Turn off the light.",
            "Alexa. Add milk to my shopping list.",
        ] {
            loop {
                speaker.say(sentence)?;
                if interact::user_choice(
                    "Confirm that Alexa recognised the sentence or repeat it",
                    &[KeyType::Enter, KeyType::Key('r')],
                )? == KeyType::Enter
                {
                    break;
                }
            }
        }
        println!("Finished setting up Alexa");

        Ok(())
    }

    fn prepare_queries(&self, queries: &mut Vec<Query>) {
        info!("Preparing queries for Alexa...");

        queries.iter_mut().for_each(|q| {
            q.text = format!("Alexa. {}", q.text);
        });
    }

    fn stop_assistant(&self, interactor: &Interactor) -> Result<(), Error> {
        info!("Telling Alexa to stop...");

        interactor.speaker.say("Alexa, stop.")?;
        interactor.listener.wait_until_silent(
            self.silence_between_interactions(),
            interactor.sensitivity,
            false,
        )?;

        Ok(())
    }

    fn reset_assistant(&self, interactor: &Interactor) -> Result<(), Error> {
        info!("Telling Alexa to stop everything...");

        let wait = || {
            interactor.listener.wait_until_silent(
                self.silence_after_talking(),
                interactor.sensitivity,
                false,
            )
        };

        interactor.speaker.say("Alexa, stop.")?;
        wait()?;
        interactor.speaker.say("Alexa, turn off the music.")?;
        wait()?;
        interactor.speaker.say("Alexa, disable all alarms.")?;
        wait()?;

        info!("Alexa has been told to stop everything");

        Ok(())
    }

    fn test_voices(&self, voices: Vec<String>) -> Result<(), Error> {
        info!("Testing Alexa voices...");

        let mut speaker = Speaker::new()?;

        for voice in voices {
            interact::user_confirmation(&format!("Test {}", voice))?;
            speaker.set_voice(&voice).unwrap();
            speaker.say("Alexa, what is my name?").unwrap();
        }

        Ok(())
    }

    fn silence_after_talking(&self) -> Duration {
        Duration::from_secs(2)
    }

    fn silence_between_interactions(&self) -> Duration {
        Duration::from_secs(5)
    }

    fn recording_timeout(&self) -> Duration {
        Duration::from_secs(120)
    }
}
