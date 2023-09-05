use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Arguments {
    #[clap(subcommand)]
    pub command: Command,
    /// Which voice assistant to interact with
    #[arg(short, long)]
    pub assistant: Option<String>,
    /// The network interface to listen on
    #[arg(short, long, default_value = "en0")]
    pub interface: String,
    /// The voice to use for speaking
    #[arg(short, long, default_value = "Zoe")]
    pub voice: String,
    /// The sensitivity to distinguish ambient noise from speech
    #[arg(short, long, default_value_t = 0.01)]
    pub sensitivity: f32,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Interact with a voice assistant
    Assistant(AssistantCommand),
    /// Listen for something that was said and optionally repeat it
    Listen(ListenCommand),
    /// Record network traffic on a specified interface
    Sniff(SniffCommand),
    /// Calibrate audio recording
    Calibrate,
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
    /// Interact with a voice assistant
    Interact(InteractionCommand),
    /// Test voice recognition with a number of voices
    Test(TestCommand),
}

#[derive(Debug, Args)]
pub struct InteractionCommand {
    /// The file with queries to ask the assistant
    pub queries: PathBuf,
}

#[derive(Debug, Args)]
pub struct TestCommand {
    #[arg(required(true))]
    /// The names of the system voices to test with
    pub voices: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ListenCommand {
    /// Optional duration in seconds to listen for. If omitted, listen until silence is detected
    #[arg(short, long)]
    pub seconds: Option<u32>,
    /// Whether to repeat the audio back
    #[arg(short, long)]
    pub parrot: bool,
    /// Where to store the recorded audio
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct SniffCommand {
    /// The duration in seconds to listen for
    #[arg(short, long, default_value_t = 5)]
    pub seconds: u32,
    /// Where to store the recorded traffic
    pub file: PathBuf,
}
