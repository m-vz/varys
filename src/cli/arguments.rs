use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Arguments {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Interact with a voice assistant.
    Assistant(AssistantCommand),
}

#[derive(Debug, Args)]
pub struct AssistantCommand {
    #[clap(subcommand)]
    pub command: AssistantSubcommand,
    /// Which voice assistant to interact with. Defaults to Siri.
    #[arg(short, long, value_name = "ASSISTANT")]
    pub assistant: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum AssistantSubcommand {
    /// Setup voice recognition.
    Setup,
}
