use std::fs;
use std::path::Path;
use std::time::Duration;

use chrono::Local;
use colored::Colorize;
use log::{info, warn};
use rand::seq::SliceRandom;

use crate::assistant::{Error, VoiceAssistant};
use crate::cli::{interact, key_type::KeyType};
use crate::listen::Listener;
use crate::recognise::{Model, Recogniser};
use crate::speak::Speaker;
use crate::{file, sniff, sniff::Sniffer};

/// The [`VoiceAssistant`] implementation for Siri. Tested with the HomePod.
pub struct Siri {}

impl Siri {
    pub const PREMIUM_VOICES: &'static [&'static str] =
        &["Ava", "Karen", "Jamie", "Matilda", "Serena", "Zoe"];
}

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

    fn interact(
        &self,
        interface: &str,
        voice: &str,
        sensitivity: f32,
        model: Model,
        queries: &Path,
    ) -> Result<(), Error> {
        info!("Interacting with Siri...");

        let mut speaker = Speaker::new()?;
        speaker.set_voice(voice)?;
        let listener = Listener::new()?;
        let recogniser = Recogniser::with_model(model)?;
        let sniffer = Sniffer::from(sniff::device_by_name(interface)?);

        if let Ok(queries) = fs::read_to_string(queries) {
            let mut queries: Vec<String> = queries
                .lines()
                .map(|q| format!("Hey Siri. {}", q))
                .collect();
            queries.shuffle(&mut rand::thread_rng());

            for query in queries {
                info!("Saying {}", query);

                let file_path =
                    format!("query-{}.pcap", Local::now().format("%Y-%m-%d-%H-%M-%S-%f"));
                let file_path = Path::new(file_path.as_str());
                let sniffer_instance = sniffer.start(file_path)?;
                speaker.say(&query, true)?;
                let mut audio =
                    listener.record_until_silent(Duration::from_secs(2), sensitivity)?;
                info!("{}", recogniser.recognise(&mut audio)?);
                info!("{}", sniffer_instance.stop()?);
                file::compress_gzip(file_path, false)?;
            }
        } else {
            warn!("Could not read queries");
        }

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
}
