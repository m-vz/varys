use crate::assistant::Assistant;
use crate::cli::arguments::{
    Arguments, AssistantCommand, AssistantSubcommand, Command, ParrotCommand, SniffCommand,
};
use crate::listen::Listener;
use crate::recognise::{Model, Recogniser};
use crate::sniff::Sniffer;
use crate::speak::Speaker;
use crate::{assistant, sniff, Error};
use clap::Parser;
use log::{debug, info};
use pcap::ConnectionStatus;
use std::time;

pub mod arguments;
pub mod interact;
pub mod key_type;

pub fn run() -> Result<(), Error> {
    let arguments = Arguments::parse();
    let assistant = assistant::from(arguments.assistant.as_deref());

    match arguments.command {
        Command::Assistant(command) => {
            assistant_command(&arguments.interface, &arguments.voice, command, assistant)
        }
        Command::Parrot(command) => parrot_command(&arguments.voice, command),
        Command::Sniff(command) => sniff_command(command),
    }
}

fn assistant_command(
    interface: &str,
    voice: &str,
    command: AssistantCommand,
    assistant: impl Assistant,
) -> Result<(), Error> {
    match command.command {
        AssistantSubcommand::Setup => assistant.setup()?,
        AssistantSubcommand::Test(test) => assistant.test(test.voices)?,
        AssistantSubcommand::Interact(command) => {
            assistant.interact(interface, voice, command.queries)?
        }
    };

    Ok(())
}

fn parrot_command(voice: &str, command: ParrotCommand) -> Result<(), Error> {
    info!("Listening...");
    let listener = Listener::new()?;
    let mut audio = if let Some(seconds) = command.seconds {
        listener.record_for(seconds)?
    } else {
        listener.record_until_silent(time::Duration::from_secs(2), 0.001)?
    };

    let mut file_path = command.file;
    file_path.set_extension("wav");
    audio.downsample(16000)?.save_to_file(file_path)?;

    info!("Recognising...");
    let recogniser = Recogniser::with_model(Model::Large)?;
    let text = recogniser.recognise(&mut audio)?;

    info!("Speaking...");
    let mut speaker = Speaker::new()?;
    speaker.set_voice(voice)?;
    speaker.say(&text, false)?;

    Ok(())
}

fn sniff_command(command: SniffCommand) -> Result<(), Error> {
    info!("Sniffing...");
    for device in sniff::devices_with_status(&ConnectionStatus::Connected)? {
        debug!("{}", Sniffer::from(device));
    }
    let sniffer = Sniffer::from(sniff::device_by_name("ap1")?);
    debug!("Using: {}", sniffer);
    let stats = sniffer.run_for(5, Some(command.file))?;
    debug!("Stats: {}", stats);

    Ok(())
}
