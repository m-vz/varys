use log::{debug, info};
use pcap::ConnectionStatus;
use std::string::ToString;
use thiserror::Error;
use varys::listen::Listener;
use varys::recognise::{Model, Recogniser};
use varys::sniff::Sniffer;
use varys::speak::Speaker;
use varys::{listen, recognise, sniff, speak};

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
}

const RECORDING_PATH: &str = "data/recordings/recorded.wav";
const VOICE: &str = "Jamie";

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    info!("Sniffing...");
    for device in sniff::devices_with_status(&ConnectionStatus::Connected)? {
        debug!("{}", Sniffer::from(device));
    }
    let sniffer = Sniffer::from(sniff::default_device()?);
    debug!("Using default: {}", sniffer);

    Ok(())
}

#[allow(unused)]
fn listen_recognise_speak() -> Result<(), Error> {
    info!("Listening...");
    let listener = Listener::new()?;
    let mut audio = listener.record(5)?;
    audio
        .downsample(16000)?
        .save_to_file(RECORDING_PATH.to_string())?;

    info!("Recognising...");
    let recogniser = Recogniser::with_model(Model::Large)?;
    let text = recogniser.recognise(&mut audio)?;

    info!("Speaking...");
    let mut speaker = Speaker::new()?;
    speaker.set_voice(VOICE)?;
    speaker.say(text, false)?;

    Ok(())
}
