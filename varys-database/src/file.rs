use std::path::{Path, PathBuf};
use std::{fs, io};

use log::debug;

use crate::database::interaction::Interaction;

pub enum DataType {
    Capture,
    Audio(String),
}

pub fn create_session_dir<P: AsRef<Path>>(data_path: P, session_id: i32) -> io::Result<PathBuf> {
    let path = session_path(data_path, session_id);

    debug!("Storing data files at {}", path.display());

    fs::create_dir_all(&path)?;

    Ok(path)
}

pub fn session_path<P: AsRef<Path>>(data_path: P, session_id: i32) -> PathBuf {
    data_path
        .as_ref()
        .join(format!("sessions/session_{}", session_id))
}

pub fn artefact_path<P: AsRef<Path>>(
    data_path: P,
    data_type: DataType,
    interaction: &Interaction,
) -> PathBuf {
    session_path(data_path, interaction.session_id).join(match data_type {
        DataType::Capture => data_file_name(interaction, "capture", "pcap"),
        DataType::Audio(prefix) => data_file_name(interaction, &format!("{prefix}-audio"), "opus"),
    })
}

fn data_file_name(interaction: &Interaction, data_type: &str, file_type: &str) -> PathBuf {
    PathBuf::from(format!(
        "s{}i{}-{}-{}.{}",
        interaction.session_id,
        interaction.id,
        data_type,
        interaction.started.format("%Y-%m-%d-%H-%M-%S-%f"),
        file_type,
    ))
}
