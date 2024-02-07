use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Arguments {
    #[clap(subcommand)]
    pub command: Command,
    /// The network interface to listen on
    #[arg(short, long, global = true, default_value = "en0")]
    pub interface: String,
    /// The voices to use for speaking, one random voice is used per session
    #[arg(short, long, global = true, default_values_t = vec!["Zoe".to_string()])]
    pub voices: Vec<String>,
    /// The sensitivity to distinguish ambient noise from speech
    #[arg(short, long, global = true, default_value_t = 0.01)]
    pub sensitivity: f32,
    /// The speech recognition model to use
    #[arg(short, long, global = true, default_value = "")]
    pub model: String,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Interact with a voice assistant
    Assistant(AssistantCommand),
    /// Listen for something that was said and optionally repeat it
    Listen(ListenCommand),
    /// Record network traffic on a specified interface
    Sniff(SniffCommand),
    /// Start varys
    Run(RunCommand),
}

#[derive(Debug, Args)]
pub struct AssistantCommand {
    /// Which voice assistant to interact with
    pub assistant: String,
    /// What to do with the assistant
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
pub struct ListenCommand {
    /// Optional duration in seconds to listen for. If omitted, listen until silence is detected
    #[arg(short, long)]
    pub duration: Option<u32>,
    /// Calibrate to the current ambient noise
    #[arg(short, long)]
    pub calibrate: bool,
    /// Whether to repeat the audio back
    #[arg(short, long)]
    pub parrot: bool,
    /// Where to store the recorded audio
    pub file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct SniffCommand {
    /// The duration in seconds to listen for
    #[arg(short, long, default_value_t = 5)]
    pub duration: u32,
    /// Where to store the recorded traffic
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct RunCommand {
    /// The MAC address of the assistant
    #[arg(long)]
    pub mac: String,
    /// Which voice assistant to interact with
    pub assistant: String,
    /// The file with queries to ask the assistant
    pub queries: PathBuf,
    /// The directory in which to store data files
    pub data_dir: PathBuf,
}
