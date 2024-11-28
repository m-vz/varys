use clap::Parser;
use log::{debug, error, info};
use std::path::Path;
use std::str::FromStr;
use std::{thread, time};
use varys_analysis::ml::data::NumericTraceDataset;
use varys_analysis::{ml, plot};
use varys_audio::listen::Listener;
use varys_audio::stt::transcriber::Transcriber;
use varys_audio::stt::Recogniser;
use varys_audio::tts::Speaker;
use varys_database::database;
use varys_database::database::interaction::Interaction;
use varys_network::address::MacAddress;
use varys_network::sniff;
use varys_network::sniff::{ConnectionStatus, Sniffer};

use crate::assistant;
use crate::assistant::interactor::Interactor;
use crate::cli::arguments::{
    AnalyseSubcommand, Arguments, AssistantCommand, AssistantSubcommand, Command, ListenCommand,
    SniffCommand,
};
use crate::dataset::DatasetSize;
use crate::error::Error;
use crate::query::Query;

pub mod arguments;
mod export;
pub mod interact;
pub mod key_type;

/// Start the cli program.
///
/// This parses the arguments passed in the command line and runs the appropriate command.
pub async fn run() -> Result<(), Error> {
    let arguments = Arguments::parse();

    match arguments.command {
        Command::Assistant(command) => assistant_command(command),
        Command::Listen(command) => listen_command(
            arguments.voices.first().ok_or(Error::NoVoiceProvided)?,
            arguments.sensitivity,
            arguments.model,
            command,
        ),
        Command::Sniff(command) => sniff_command(&arguments.interface, command),
        Command::Run(command) => {
            run_command(
                &arguments.interface,
                arguments.voices,
                arguments.sensitivity,
                arguments.model,
                command,
            )
            .await
        }
        Command::Analyse(command) => {
            analyse_command(command.dataset, command.command, &arguments.interface).await
        }
        Command::Export(export_command) => {
            export_command
                .format
                .export(
                    export_command.data_dir,
                    &export_command.dataset,
                    assistant::from(&export_command.assistant),
                )
                .await
        }
    }
}

fn assistant_command(command: AssistantCommand) -> Result<(), Error> {
    let assistant = assistant::from(command.assistant.as_str());

    match command.command {
        AssistantSubcommand::Setup => assistant.setup()?,
        AssistantSubcommand::Test(test) => assistant.test_voices(test.voices)?,
    };

    Ok(())
}

fn listen_command<P: AsRef<Path>>(
    voice: &str,
    sensitivity: f32,
    model: P,
    command: ListenCommand,
) -> Result<(), Error> {
    if command.calibrate {
        calibrate()
    } else {
        listen(voice, sensitivity, model, command)
    }
}

fn calibrate() -> Result<(), Error> {
    interact::user_confirmation("Calibration will record the average ambient noise. Stay quiet for five seconds. To begin, press")?;

    let average = Listener::new()?.calibrate()?;
    println!("The average ambient noise is {average}");

    Ok(())
}

fn listen<P: AsRef<Path>>(
    voice: &str,
    sensitivity: f32,
    model: P,
    command: ListenCommand,
) -> Result<(), Error> {
    info!("Listening...");
    let listener = Listener::new()?;
    let mut audio = if let Some(seconds) = command.duration {
        listener.record_for(seconds, sensitivity)?
    } else {
        listener.record_until_silent(time::Duration::from_secs(2), sensitivity)?
    };
    audio.downsample(16000)?;
    if let Some(file) = command.file {
        varys_audio::file::write_audio(&file, &audio)?;
    }

    if command.parrot {
        info!("Recognising...");
        let recogniser = Recogniser::with_model_path(&model.as_ref().to_string_lossy())?;
        let text = recogniser.recognise(&mut audio)?;

        info!("Speaking...");
        let speaker = Speaker::with_voice(voice)?;
        speaker.say(&text)?;
    }

    Ok(())
}

fn sniff_command(interface: &str, command: SniffCommand) -> Result<(), Error> {
    info!("Sniffing...");

    for device in sniff::devices_with_status(&ConnectionStatus::Connected)? {
        debug!("{}", Sniffer::from(device));
    }
    let sniffer = Sniffer::from(sniff::device_by_name(interface)?);
    debug!("Using: {sniffer}");
    let stats = sniffer.run_for(5, &command.file)?;
    debug!("Stats: {stats}");

    Ok(())
}

async fn run_command<P: AsRef<Path>>(
    interface: &str,
    voices: Vec<String>,
    sensitivity: f32,
    model: P,
    command: arguments::RunCommand,
) -> Result<(), Error> {
    let mut interactor = Interactor::new(
        interface.to_string(),
        voices,
        sensitivity,
        model.as_ref().to_string_lossy().to_string(),
        command.data_dir,
        command.mac,
    )?;
    let assistant = assistant::from(command.assistant.as_str());
    let mut queries = Query::read_toml(&command.queries)?;
    assistant.prepare_queries(&mut queries);

    loop {
        let (transcriber, transcriber_handle) = Transcriber::new(Recogniser::with_model_path(
            &model.as_ref().to_string_lossy(),
        )?);

        let _ = thread::spawn(move || transcriber.start());

        if let Err(error) = interactor
            .start(&mut queries, assistant.as_ref(), transcriber_handle)
            .await
        {
            error!("A session did not complete successfully: {error}");
        }
    }
}

async fn analyse_command(
    dataset_size: DatasetSize,
    analyse_subcommand: AnalyseSubcommand,
    interface: &str,
) -> Result<(), Error> {
    match analyse_subcommand {
        AnalyseSubcommand::Train { data_dir } => {
            ml::train(data_dir, get_filtered_interactions(&dataset_size).await?)?
        }
        AnalyseSubcommand::Test { data_dir } => ml::test_dataset(data_dir)?,
        AnalyseSubcommand::Demo { data_dir, mac } => demo(data_dir, interface, mac)?,
        AnalyseSubcommand::CompileLogs { data_dir, id } => ml::compile_all_logs(data_dir, &id)?,
        AnalyseSubcommand::Plot { data_dir } => {
            let mut dataset = NumericTraceDataset::new(
                &data_dir,
                get_filtered_interactions(&dataset_size).await?,
            )?;
            dataset.resize_all(475).shuffle();

            plot::plot_queries(&data_dir, dataset_size.queries(), &dataset);
        }
    }

    Ok(())
}

fn demo<P: AsRef<Path>>(data_dir: P, interface: &str, address: String) -> Result<(), Error> {
    let sniffer = Sniffer::from(sniff::device_by_name(interface)?);
    let capture_path = data_dir.as_ref().join("captures/demo.pcap");
    let data_dir = data_dir.as_ref().to_path_buf();

    interact::user_confirmation("Starting the demo.")?;
    interact::user_confirmation(&format!(
        "The demo capture will be stored at {}.",
        capture_path.display()
    ))?;
    let sniffer = sniffer.start(&capture_path)?;
    interact::user_confirmation("Confirm when the voice assistant has finished speaking.")?;
    let _ = sniffer.stop()?;
    let output = ml::test_single(&data_dir, &capture_path, &MacAddress::from_str(&address)?)?;
    println!("{output:?}");

    Ok(())
}

async fn get_filtered_interactions(dataset_size: &DatasetSize) -> Result<Vec<Interaction>, Error> {
    let connection = database::connect().await?;
    let all_interactions = Interaction::get_all(&connection).await?;
    log::info!("Fetched all interactions: {}", all_interactions.len()); // Debugging
    Ok(dataset_size.filter(all_interactions))
}
