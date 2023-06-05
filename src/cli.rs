use crate::assistant::Assistant;
use crate::cli::arguments::{Arguments, AssistantCommand, AssistantSubcommand, Command};
use crate::{assistant, Error};
use clap::Parser;

pub mod arguments;
pub mod interact;
pub mod key_type;

pub fn run() -> Result<(), Error> {
    let arguments = Arguments::parse();
    let assistant = assistant::from(arguments.assistant);

    match arguments.command {
        Command::Assistant(command) => assistant_command(command, assistant),
    }
}

fn assistant_command(command: AssistantCommand, assistant: impl Assistant) -> Result<(), Error> {
    match command.command {
        AssistantSubcommand::Setup => assistant.setup()?,
        AssistantSubcommand::Test(test) => assistant.test(test.voices)?,
    };

    Ok(())
}
