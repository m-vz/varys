use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    str::FromStr,
};

use clap::ValueEnum;
use varys_analysis::trace::TrafficTrace;
use varys_database::file;
use varys_network::{address::MacAddress, packet};

use crate::{cli, dataset::DatasetSize, error::Error};

#[derive(ValueEnum, Clone, Debug)]
pub enum ExportType {
    Wang,
    Ahmed,
}

impl ExportType {
    pub async fn export<P: AsRef<Path>>(
        &self,
        data_dir: P,
        dataset_size: &DatasetSize,
    ) -> Result<(), Error> {
        let export_dir = data_dir
            .as_ref()
            .join("ml/export")
            .join(match self {
                ExportType::Wang => "wang",
                ExportType::Ahmed => "ahmed",
            })
            .join(dataset_size.to_string());

        match self {
            ExportType::Wang => {
                Self::export_wang(data_dir.as_ref(), &export_dir, dataset_size).await
            }
            ExportType::Ahmed => todo!(),
        }
    }

    async fn export_wang<P: AsRef<Path>>(
        data_dir: P,
        export_dir: P,
        dataset_size: &DatasetSize,
    ) -> Result<(), Error> {
        let interactions = cli::get_filtered_interactions(dataset_size).await?;

        log::info!("Number of interactions: {:?}", interactions.len());

        for (label, query) in dataset_size
            .queries()
            .iter()
            .enumerate()
            .map(|(index, query)| (index + 1, query))
        {
            let query_dir = export_dir.as_ref().join(label.to_string());
            fs::create_dir_all(&query_dir)?;

            log::info!("Exporting interactions for \"{query}\" to {query_dir:?}");

            for (index, interaction) in interactions
                .iter()
                .filter(|interaction| {
                    **query == interaction.query && interaction.capture_file.is_some()
                })
                .enumerate()
            {
                let mac_address =
                    MacAddress::from_str(&interaction.assistant_mac).expect("Cannot load MAC");
                let capture_path = interaction
                    .capture_file
                    .clone()
                    .map(|path| file::session_path(&data_dir, interaction.session_id).join(path))
                    .expect("Cannot load capture path");
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
                    writeln!(csv, "{timestamp:?},{size:.1},{direction:.1}")?;
                }

                log::trace!("Exported {interaction_path:?}");
            }
        }

        Ok(())
    }
}
