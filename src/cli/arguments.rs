use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Arguments {
    #[clap(subcommand)]
    pub command: Command,
    /// Which voice assistant to interact with
    #[arg(short, long)]
    pub assistant: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Interact with a voice assistant
    Assistant(AssistantCommand),
    /// Listen for something that was said and repeat it
    Parrot(ParrotCommand),
    /// Record network traffic on a specified interface
    Sniff(SniffCommand),
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

#[derive(Debug, Args)]
pub struct ParrotCommand {
    /// Optional duration in seconds to listen for. If omitted, listen until silence is detected
    #[arg(short, long)]
    pub seconds: Option<u32>,
    /// The voice to use for speaking
    #[arg(short, long, default_value = "Zoe")]
    pub voice: String,
    /// Where to store the recorded audio
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct SniffCommand {
    /// The duration in seconds to listen for
    #[arg(short, long, default_value_t = 5)]
    pub seconds: u32,
    /// The network interface to listen on
    #[arg(short, long, default_value = "en0")]
    pub interface: String,
    /// Where to store the recorded traffic
    pub file: PathBuf,
}
