use std::time::Duration;

use async_trait::async_trait;
use colored::Colorize;
use log::info;
use rand::seq::SliceRandom;

use crate::assistant::interactor::Interactor;
use crate::assistant::{Error, VoiceAssistant};
use crate::cli::{interact, key_type::KeyType};
use crate::database::query::Query;
use crate::speak::Speaker;

/// The [`VoiceAssistant`] implementation for Siri. Tested with the HomePod.
pub struct Siri {}

impl Siri {
    pub const PREMIUM_VOICES: &'static [&'static str] =
        &["Ava", "Karen", "Jamie", "Matilda", "Serena", "Zoe"];
}

#[async_trait]
impl VoiceAssistant for Siri {
    fn name(&self) -> String {
        "Siri".to_string()
    }

    fn setup(&self) -> Result<(), Error> {
        info!("Starting Siri setup...");

        let mut speaker = Speaker::new()?;

        let voice = interact::user_input(
            &format!(
                "Choose the voice to set up (The highest quality voices on macOS are {}):",
                Siri::PREMIUM_VOICES.join(", ")
            ),
            |i| speaker.set_voice(i).is_ok(),
            "Voice not found, enter a voice that can be used on this system:",
        )?;
        println!("Setting up Siri for {}", voice);
        interact::user_confirmation(
            "This requires a number of sentences to be said. To continue, press",
        )?;
        interact::user_confirmation(
            "Make sure your iOS device is close enough to this computer to hear it.",
        )?;
        interact::user_confirmation(&format!(
            "On your iOS device, go to {} > {} and enable {} (you might have to disable it first)",
            "Settings".bright_blue(),
            "Siri & Search".bright_blue(),
            "Listen for \"Hey Siri\"".bright_blue()
        ))?;
        interact::user_confirmation(&format!(
            "The sentences will now be said. Press {} on your device and then",
            "Continue".bright_blue()
        ))?;
        for sentence in [
            "Hey Siri",
            "Hey Siri. Send a message.",
            "Hey Siri. How's the weather today?",
            "Hey Siri. Set a timer for three minutes.",
            "Hey Siri. Play some music.",
        ] {
            loop {
                speaker.say(sentence, true)?;
                if interact::user_choice(
                    "Confirm that Siri recognised the sentence or repeat it",
                    &[KeyType::Enter, KeyType::Key('r')],
                )? == KeyType::Enter
                {
                    break;
                }
            }
        }
        println!("Finished setting up Siri");

        Ok(())
    }

    async fn interact(
        &self,
        mut interactor: Interactor,
        mut queries: Vec<Query>,
    ) -> Result<(), Error> {
        info!("Interacting with Siri...");

        queries.iter_mut().for_each(|q| {
            q.text = format!("Hey Siri. {}", q.text);
        });

        loop {
            queries.shuffle(&mut rand::thread_rng());
            interactor = interactor
                .begin_session(self)
                .await?
                .start(&queries)
                .await?;
        }
    }

    fn stop_assistant(&self, interactor: &mut Interactor) -> Result<(), Error> {
        info!("Telling Siri to stop...");

        interactor.speaker.say("Hey Siri, stop.", true)?;
        interactor.listener.wait_until_silent(
            self.silence_after_talking(),
            interactor.sensitivity,
            false,
        )?;

        Ok(())
    }

    fn reset_assistant(&self, interactor: &mut Interactor) -> Result<(), Error> {
        info!("Telling Siri to stop everything...");

        let wait = || {
            interactor.listener.wait_until_silent(
                self.silence_after_talking(),
                interactor.sensitivity,
                true,
            )
        };

        interactor.speaker.say("Hey Siri, stop.", true)?;
        wait()?;
        interactor
            .speaker
            .say("Hey Siri, turn off the music.", true)?;
        wait()?;
        interactor
            .speaker
            .say("Hey Siri, disable all alarms.", true)?;
        wait()?;

        info!("Siri has been told to stop everything");

        Ok(())
    }

    fn test_voices(&self, voices: Vec<String>) -> Result<(), Error> {
        info!("Testing Siri voices...");

        let mut speaker = Speaker::new()?;

        for voice in voices {
            interact::user_confirmation(&format!("Test {}", voice))?;
            speaker.set_voice(&voice).unwrap();
            speaker.say("Hey Siri, what is my name?", true).unwrap();
        }

        Ok(())
    }

    fn silence_after_talking(&self) -> Duration {
        Duration::from_secs(2)
    }

    fn silence_between_interactions(&self) -> Duration {
        Duration::from_secs(10)
    }

    fn recording_timeout(&self) -> Duration {
        Duration::from_secs(120)
    }
}
