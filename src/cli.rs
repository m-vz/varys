use std::time;

use clap::Parser;
use log::{debug, info};
use pcap::ConnectionStatus;

use crate::cli::arguments::{
    Arguments, AssistantCommand, AssistantSubcommand, Command, ListenCommand, SniffCommand,
};
use crate::error::Error;
use crate::listen::Listener;
use crate::recognise::{Model, Recogniser};
use crate::speak::Speaker;
use crate::{assistant, assistant::VoiceAssistant, file};
use crate::{sniff, sniff::Sniffer};

pub mod arguments;
pub mod interact;
pub mod key_type;

/// Start the cli program.
///
/// This parses the arguments passed in the command line and runs the appropriate command.
pub fn run() -> Result<(), Error> {
    let arguments = Arguments::parse();
    let model = Model::from(arguments.model);

    match arguments.command {
        Command::Assistant(command) => assistant_command(
            &arguments.interface,
            &arguments.voice,
            arguments.sensitivity,
            model,
            command,
        ),
        Command::Listen(command) => {
            listen_command(&arguments.voice, arguments.sensitivity, model, command)
        }
        Command::Sniff(command) => sniff_command(command),
        Command::Calibrate => calibrate_command(),
    }
}

fn assistant_command(
    interface: &str,
    voice: &str,
    sensitivity: f32,
    model: Model,
    command: AssistantCommand,
) -> Result<(), Error> {
    let assistant = assistant::from(command.assistant.as_str());

    match command.command {
        AssistantSubcommand::Setup => assistant.setup()?,
        AssistantSubcommand::Test(test) => assistant.test_voices(test.voices)?,
        AssistantSubcommand::Interact(command) => {
            assistant.interact(interface, voice, sensitivity, model, &command.queries)?
        }
    };

    Ok(())
}

fn listen_command(
    voice: &str,
    sensitivity: f32,
    model: Model,
    command: ListenCommand,
) -> Result<(), Error> {
    info!("Listening...");
    let listener = Listener::new()?;
    let mut audio = if let Some(seconds) = command.seconds {
        listener.record_for(seconds, sensitivity)?
    } else {
        listener.record_until_silent(time::Duration::from_secs(2), sensitivity)?
    };
    audio.downsample(16000)?;
    file::audio::write_audio(&command.file, &audio)?;

    if command.parrot {
        info!("Recognising...");
        let recogniser = Recogniser::with_model(model)?;
        let text = recogniser.recognise(&mut audio)?;

        info!("Speaking...");
        let mut speaker = Speaker::new()?;
        speaker.set_voice(voice)?;
        speaker.say(&text, false)?;
    }

    Ok(())
}

fn sniff_command(command: SniffCommand) -> Result<(), Error> {
    info!("Sniffing...");
    for device in sniff::devices_with_status(&ConnectionStatus::Connected)? {
        debug!("{}", Sniffer::from(device));
    }
    let sniffer = Sniffer::from(sniff::device_by_name("ap1")?);
    debug!("Using: {sniffer}");
    let stats = sniffer.run_for(5, &command.file)?;
    debug!("Stats: {stats}");
    file::compress_gzip(&command.file, true)?;

    Ok(())
}

fn calibrate_command() -> Result<(), Error> {
    interact::user_confirmation("Calibration will record the average ambient noise. Stay quiet for five seconds. To begin, press")?;

    let average = Listener::new()?.calibrate()?;
    info!("The average ambient noise is {average}");

    Ok(())
}
