use std::{thread, time};

use clap::Parser;
use log::{debug, error, info};
use pcap::ConnectionStatus;

use crate::{assistant, assistant::VoiceAssistant, file};
use crate::{sniff, sniff::Sniffer};
use crate::assistant::interactor::Interactor;
use crate::cli::arguments::{
    Arguments, AssistantCommand, AssistantSubcommand, Command, ListenCommand, SniffCommand,
};
use crate::database::query::Query;
use crate::error::Error;
use crate::listen::Listener;
use crate::recognise::{Model, Recogniser};
use crate::recognise::transcriber::Transcriber;
use crate::speak::Speaker;

pub mod arguments;
pub mod interact;
pub mod key_type;

/// Start the cli program.
///
/// This parses the arguments passed in the command line and runs the appropriate command.
pub async fn run() -> Result<(), Error> {
    let arguments = Arguments::parse();
    let model = Model::from(arguments.model);

    match arguments.command {
        Command::Assistant(command) => assistant_command(command),
        Command::Listen(command) => listen_command(
            arguments.voices.first().ok_or(Error::NoVoiceProvided)?,
            arguments.sensitivity,
            model,
            command,
        ),
        Command::Sniff(command) => sniff_command(&arguments.interface, command),
        Command::Run(command) => {
            run_command(
                &arguments.interface,
                arguments.voices,
                arguments.sensitivity,
                model,
                command,
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

fn listen_command(
    voice: &str,
    sensitivity: f32,
    model: Model,
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

fn listen(
    voice: &str,
    sensitivity: f32,
    model: Model,
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
        file::audio::write_audio(&file, &audio)?;
    }

    if command.parrot {
        info!("Recognising...");
        let recogniser = Recogniser::with_model(model)?;
        let text = recogniser.recognise(&mut audio)?;

        info!("Speaking...");
        let speaker = Speaker::with_voice(voice)?;
        speaker.say(&text, false)?;
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
    file::compress_gzip(&command.file, true)?;

    Ok(())
}

async fn run_command(
    interface: &str,
    voices: Vec<String>,
    sensitivity: f32,
    model: Model,
    command: arguments::RunCommand,
) -> Result<(), Error> {
    let assistant = assistant::from(command.assistant.as_str());
    let mut interactor = Interactor::new(
        interface.to_string(),
        voices,
        sensitivity,
        model,
        command.data_dir,
    )?;
    let queries = Query::read_toml(&command.queries)?;

    loop {
        let (transcriber, transcriber_handle) = Transcriber::new(Recogniser::with_model(model)?);

        let _ = thread::spawn(move || transcriber.start());

        if let Err(error) = assistant
            .interact(&mut interactor, queries.clone(), transcriber_handle)
            .await
        {
            error!("A session did not complete successfully: {error}");
        }
    }
}
