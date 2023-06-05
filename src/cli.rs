use crate::assistant::Assistant;
use crate::cli::arguments::{
    Arguments, AssistantCommand, AssistantSubcommand, Command, ParrotCommand,
};
use crate::listen::Listener;
use crate::recognise::Recogniser;
use crate::speak::Speaker;
use crate::{assistant, speak, Error};
use clap::Parser;
use log::{info, warn};
use std::time;

pub mod arguments;
pub mod interact;
pub mod key_type;

pub fn run() -> Result<(), Error> {
    let arguments = Arguments::parse();
    let assistant = assistant::from(arguments.assistant);

    match arguments.command {
        Command::Assistant(command) => assistant_command(command, assistant),
        Command::Parrot(command) => parrot_command(command),
    }
}

fn assistant_command(command: AssistantCommand, assistant: impl Assistant) -> Result<(), Error> {
    match command.command {
        AssistantSubcommand::Setup => assistant.setup()?,
        AssistantSubcommand::Test(test) => assistant.test(test.voices)?,
    };

    Ok(())
}

fn parrot_command(command: ParrotCommand) -> Result<(), Error> {
    info!("Listening...");
    let listener = Listener::new()?;
    let mut audio = if let Some(seconds) = command.seconds {
        listener.record_for(seconds)?
    } else {
        listener.record_until_silent(time::Duration::from_secs(2), 0.001)?
    };
    if let Ok(file_path) = command.file.into_os_string().into_string() {
        audio.downsample(16000)?.save_to_file(file_path)?;
    } else {
        warn!("Could not convert file path to a valid string")
    }

    info!("Recognising...");
    let recogniser = Recogniser::with_model(crate::recognise::Model::Large)?;
    let text = recogniser.recognise(&mut audio)?;

    info!("Speaking...");
    let mut speaker = Speaker::new()?;
    speaker.set_voice(&command.voice)?;
    speaker.say(&text, false)?;

    Ok(())
}
