use crate::cli::arguments::{Arguments, AssistantCommand, AssistantSubcommand, Command};
use crate::{assistant, Error};
use assistant::Setup;
use clap::Parser;
use log::debug;

pub mod arguments;
pub mod interact;
pub mod key_type;

pub fn run() -> Result<(), Error> {
    let arguments = Arguments::parse();
    debug!("{:?}", arguments);

    match arguments.command {
        Command::Assistant(command) => assistant(command),
    }
}

fn assistant(command: AssistantCommand) -> Result<(), Error> {
    let assistant = assistant::from(command.assistant);

    match command.command {
        AssistantSubcommand::Setup => assistant.setup()?,
    };

    Ok(())
}
