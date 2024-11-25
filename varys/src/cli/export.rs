use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    str::FromStr,
};

use chrono::{DateTime, Utc};
use clap::ValueEnum;
use regex::Regex;
use serde::Serialize;
use varys_analysis::trace::TrafficTrace;
use varys_database::{database::interaction::Interaction, file};
use varys_network::{address::MacAddress, packet};

use crate::{assistant::VoiceAssistant, cli, dataset::DatasetSize, error::Error};

#[derive(ValueEnum, Clone, Debug)]
pub enum ExportType {
    Wang,
    Ahmed,
}

#[derive(Serialize, Clone, Debug)]
pub struct AhmedInteraction {
    va: String,
    invoke_phrase: String,
    wake_word: String,
    #[serde(default)]
    audio_fp: String,
    label: String,
    start_time: f64,
    end_time: f64,
    #[serde(default)]
    validate_record: String,
    #[serde(default)]
    va_activity_data: String,
    complete: bool,
}

impl ExportType {
    pub async fn export<P: AsRef<Path>>(
        &self,
        data_dir: P,
        dataset_size: &DatasetSize,
        voice_assistant: Box<dyn VoiceAssistant>,
    ) -> Result<(), Error> {
        let export_dir = data_dir
            .as_ref()
            .join("ml/export")
            .join(match self {
                ExportType::Wang => "wang",
                ExportType::Ahmed => "ahmed",
            })
            .join(dataset_size.to_string());

        log::info!("Export directory: {:?}", export_dir);

        match self {
            ExportType::Wang => {
                Self::export_wang(data_dir.as_ref(), &export_dir, dataset_size).await
            }
            ExportType::Ahmed => {
                Self::export_ahmed(
                    data_dir.as_ref(),
                    &export_dir,
                    dataset_size,
                    voice_assistant,
                )
                .await
            }
        }
    }

    async fn export_ahmed<P: AsRef<Path>>(
        data_dir: P,
        export_dir: P,
        dataset_size: &DatasetSize,
        voice_assistant: Box<dyn VoiceAssistant>,
    ) -> Result<(), Error> {
        let interactions = Self::get_interactions(dataset_size).await?;
        let valid_greetings = vec!["Hey Siri. ", "Alexa. "];

        log::info!("Loaded interactions: {}", interactions.len());

        let interactions_dir = export_dir
            .as_ref()
            .join("invoke_records")
            .join(voice_assistant.name());
        let captures_dir = export_dir.as_ref().join("captures");

        log::info!("Creating captures directory: {:?}", captures_dir);
        fs::create_dir_all(&captures_dir)?;

        for query in dataset_size.queries().iter() {
            let valid_queries: Vec<String> = valid_greetings.iter()
                .map(|greeting| format!("{}{}", greeting, query))
                .collect();

            let query_stripped = query
                .strip_prefix(&format!("{}. ", voice_assistant.wake_word()))
                .unwrap_or(query);
            let label = query_stripped.to_lowercase().replace(' ', "-");
            let label = Regex::new(r"[^a-zA-Z0-9\-]")
                .expect("Invalid label regex")
                .replace_all(&label, "")
                .into_owned();
            let query_dir = interactions_dir.join(&label);

            log::info!("Creating directory for query: {:?}", query_dir);
            fs::create_dir_all(&query_dir)?;

            log::info!("Exporting interactions for \"{}\" to {:?}", query, query_dir);

            for interaction in interactions.iter().filter(|interaction| {
                valid_queries.iter().any(|valid_query| interaction.query == *valid_query) && interaction.capture_file.is_some()
            }) {
                log::info!("Processing interaction: {:?}", interaction.id);

                if let Some(ended) = interaction.ended {
                    if let Some(capture_file) = &interaction.capture_file {
                        let original_capture_path =
                            file::session_path(&data_dir, interaction.session_id)
                                .join(capture_file);
                        let capture_path = captures_dir.join(
                            original_capture_path
                                .file_name()
                                .unwrap_or_else(|| panic!("Invalid capture file: {:?}", capture_file)),
                        );

                        log::trace!("Copying from {:?} to {:?}", original_capture_path, capture_path);
                        if let Err(e) = fs::copy(&original_capture_path, &capture_path) {
                            log::error!("Failed to copy capture file: {}", e);
                        }
                    }

                    let interaction_path = query_dir.join(format!(
                        "ir_V_{}.json",
                        interaction.started.format("%Y-%m-%dT%H:%M:%S%.6f")
                    ));
                    let ahmed_interaction = AhmedInteraction {
                        va: voice_assistant.name(),
                        invoke_phrase: query_stripped.to_string(),
                        wake_word: voice_assistant.wake_word(),
                        audio_fp: String::default(),
                        label: label.clone(),
                        start_time: Self::datetime_to_timestamp(interaction.started),
                        end_time: Self::datetime_to_timestamp(ended),
                        validate_record: String::default(),
                        va_activity_data: String::default(),
                        complete: true,
                    };

                    log::trace!("Writing interaction to: {:?}", interaction_path);
                    if let Err(error) = File::create(&interaction_path)
                        .map(|file| serde_json::to_writer_pretty(file, &ahmed_interaction))?
                    {
                        log::error!("Could not write interaction file at {:?}: {}", interaction_path, error);
                    }

                    log::trace!("Exported {:?}", interaction_path);
                }
            }
        }

        Ok(())
    }

    async fn export_wang<P: AsRef<Path>>(
        data_dir: P,
        export_dir: P,
        dataset_size: &DatasetSize,
    ) -> Result<(), Error> {
        let interactions = Self::get_interactions(dataset_size).await?;
        log::info!("Loaded interactions: {}", interactions.len());

        for (label, query) in dataset_size
            .queries()
            .iter()
            .enumerate()
            .map(|(index, query)| (index + 1, query))
        {
            let query_dir = export_dir.as_ref().join(label.to_string());
            fs::create_dir_all(&query_dir)?;

            log::info!("Exporting interactions for \"{}\" to {:?}", query, query_dir);

            for (index, interaction) in interactions
                .iter()
                .filter(|interaction| {
                    **query == interaction.query && interaction.capture_file.is_some()
                })
                .enumerate()
            {
                log::info!("Processing interaction: {:?}", interaction.id);
                let mac_address =
                    MacAddress::from_str(&interaction.assistant_mac).expect("Cannot load MAC");
                let capture_path = interaction
                    .capture_file
                    .clone()
                    .map(|path| file::session_path(&data_dir, interaction.session_id).join(path))
                    .expect("Cannot load capture path");

                log::info!("Loading packets from capture file: {:?}", capture_path);
                let packets = packet::load_packets(capture_path).expect("Could not load packets");

                let traffic_trace = TrafficTrace::try_from(packets)
                    .map(|trace| trace.as_wang_traffic_trace(&mac_address))
                    .map_err(|_err| varys_analysis::error::Error::CannotLoadTrace)?;

                let interaction_path = query_dir.join(format!(
                    "{}_??_varys_{}_.csv",
                    query.replace(' ', "_"),
                    index
                ));
                let mut csv = File::create(&interaction_path)?;

                writeln!(csv, "time,size,direction")?;
                for (timestamp, size, direction) in traffic_trace.0 {
                    writeln!(csv, "{:?},{:.1},{:.1}", timestamp, size, direction)?;
                }

                log::trace!("Exported {:?}", interaction_path);
            }
        }

        Ok(())
    }

    fn datetime_to_timestamp(datetime: DateTime<Utc>) -> f64 {
        datetime.timestamp() as f64 + datetime.timestamp_subsec_nanos() as f64 * 1e-9
    }

    async fn get_interactions(dataset_size: &DatasetSize) -> Result<Vec<Interaction>, Error> {
        let interactions = cli::get_filtered_interactions(dataset_size).await?;
        log::info!("Number of interactions: {}", interactions.len());
        Ok(interactions)
    }
}