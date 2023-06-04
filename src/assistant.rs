use crate::cli::interact;
use crate::cli::key_type::KeyType;
use crate::speak::Speaker;
use crate::{cli, speak};
use colored::Colorize;
use log::info;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    CliIo(#[from] cli::interact::Error),
    #[error(transparent)]
    Speaker(#[from] speak::Error),
}

pub trait VoiceAssistant {
    fn setup(&self) -> Result<(), Error>;
}

pub struct Siri {}

impl Siri {
    pub const PREMIUM_VOICES: &'static [&'static str] =
        &["Ava", "Karen", "Jamie", "Matilda", "Serena", "Zoe"];
}

impl VoiceAssistant for Siri {
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
}
