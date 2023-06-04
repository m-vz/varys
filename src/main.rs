use log::{debug, info};
use pcap::ConnectionStatus;
use std::string::ToString;
use std::time::Duration;
use thiserror::Error;
use varys::assistant::{Siri, VoiceAssistant};
use varys::cli::interact;
use varys::listen::Listener;
use varys::recognise::{Model, Recogniser};
use varys::sniff::Sniffer;
use varys::speak::Speaker;
use varys::{assistant, listen, recognise, sniff, speak};

#[derive(Error, Debug)]
enum Error {
    #[error(transparent)]
    Speaker(#[from] speak::Error),
    #[error(transparent)]
    Listener(#[from] listen::Error),
    #[error(transparent)]
    Audio(#[from] listen::audio::Error),
    #[error(transparent)]
    Recogniser(#[from] recognise::Error),
    #[error(transparent)]
    Sniffer(#[from] sniff::Error),
    #[error(transparent)]
    Assistant(#[from] assistant::Error),
    #[error(transparent)]
    Cli(#[from] interact::Error),
}

const RECORDING_PATH: &str = "data/recordings/recorded.wav";
const PCAP_PATH: &str = "data/captures/captured.pcap";
const VOICE: &str = "Jamie";

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    test_voice_recognition()
}

#[allow(unused)]
fn setup_siri() -> Result<(), Error> {
    let siri = Siri {};
    siri.setup()?;

    Ok(())
}

#[allow(unused)]
fn sniff() -> Result<(), Error> {
    info!("Sniffing...");
    for device in sniff::devices_with_status(&ConnectionStatus::Connected)? {
        debug!("{}", Sniffer::from(device));
    }
    let sniffer = Sniffer::from(sniff::device_by_name("ap1")?);
    debug!("Using: {}", sniffer);
    let stats = sniffer.run_for(5, Some(PCAP_PATH))?;
    debug!("Stats: {}", stats);

    Ok(())
}

#[allow(unused)]
fn listen_recognise_speak() -> Result<(), Error> {
    info!("Listening...");
    let listener = Listener::new()?;
    let mut audio = listener.record_until_silent(Duration::from_secs(2), 0.001)?;
    audio
        .downsample(16000)?
        .save_to_file(RECORDING_PATH.to_string())?;

    info!("Recognising...");
    let recogniser = Recogniser::with_model(Model::Large)?;
    let text = recogniser.recognise(&mut audio)?;

    info!("Speaking...");
    let mut speaker = Speaker::new()?;
    speaker.set_voice(VOICE)?;
    speaker.say(&text, false)?;

    Ok(())
}

#[allow(unused)]
fn test_voice_recognition() -> Result<(), Error> {
    info!("Testing Siri voices...");

    let mut speaker = Speaker::new()?;

    for voice in ["Karen", "Isha", "Zoe", "Jamie"] {
        interact::user_confirmation(&format!("Test {}", voice))?;
        speaker.set_voice(voice).unwrap();
        speaker.say("Hey Siri, what is my name?", true).unwrap();
    }

    Ok(())
}
