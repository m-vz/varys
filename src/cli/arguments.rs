use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Arguments {
    #[clap(subcommand)]
    pub command: Command,
    /// Which voice assistant to interact with
    #[arg(short, long, value_name = "ASSISTANT")]
    pub assistant: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Interact with a voice assistant
    Assistant(AssistantCommand),
}

#[derive(Debug, Args)]
pub struct AssistantCommand {
    #[clap(subcommand)]
    pub command: AssistantSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AssistantSubcommand {
    /// Setup voice recognition
    Setup,
    /// Test voice recognition with a number of voices
    Test(TestCommand),
}

#[derive(Debug, Args)]
pub struct TestCommand {
    #[arg(required(true))]
    /// The names of the system voices to test with
    pub voices: Vec<String>,
}
