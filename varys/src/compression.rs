use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::{fs, fs::File};

use crate::error::Error;
use flate2::{Compression, GzBuilder};
use log::info;

/// Compress a file into a gzip wrapper.
///
/// The compressed file is written to the same path as the uncompressed one.
///
/// Returns an error if the compressed file could not be created or written.
///
/// # Arguments
///
/// * `file_path`: The path to the file to compress.
/// * `keep`: Whether to keep the uncompressed file.
///
/// Returns the path to the compressed file.
///
/// # Examples
///
/// This will try to compress `text.txt` into `text.txt.gz`, keeping the original:
///
/// ```no_run
/// # use std::path::Path;
/// # use varys::compression;
/// let file_path_compressed = compression::compress_gzip(Path::new("text.txt"), true).unwrap();
/// ```
pub fn compress_gzip(file_path: &Path, keep: bool) -> Result<PathBuf, Error> {
    let file = File::open(file_path)?;
    let reader = BufReader::with_capacity(100, file);

    info!("Compressing {:?} using gzip", file_path);

    let mut file_path_gz = file_path.to_owned().into_os_string();
    file_path_gz.push(".gz");
    let file_gz = File::create(Path::new(file_path_gz.as_os_str()))?;
    let mut encoder = GzBuilder::new().write(file_gz, Compression::default());

    reader.bytes().for_each(|b| {
        if let Ok(byte) = b {
            let _ = encoder.write_all(&[byte]);
        }
    });
    encoder.finish()?;

    if !keep {
        fs::remove_file(file_path)?;
    }

    Ok(PathBuf::from(file_path_gz))
}
