use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::{fs, fs::File};

use flate2::{Compression, GzBuilder};
use hound::WavSpec;
use log::debug;

use crate::error::Error;
use crate::listen::audio::AudioData;

/// Compress a file into a gzip wrapper.
///
/// The compressed file is written to the same path as the uncompressed one.
///
/// This returns an error if the compressed file could not be created or written.
///
/// # Arguments
///
/// * `file_path`: The path to the file to compress.
/// * `keep`: Whether to keep the uncompressed file.
///
/// # Examples
///
/// This will try to compress `text.txt` into `text.txt.gz`, keeping the original:
///
/// ```no_run
/// # use std::path::Path;
/// # use varys::file;
/// file::compress_gzip(Path::new("text.txt"), true).unwrap();
/// ```
pub fn compress_gzip(file_path: &Path, keep: bool) -> Result<(), Error> {
    let file = File::open(file_path)?;
    let reader = BufReader::with_capacity(100, file);

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

    Ok(())
}

/// Save audio data to a `.wav` file.
///
/// Returns an error if the file could not be written.
///
/// # Arguments
///
/// * `file_path`: Where to save the file. The extension `.wav` will be added if it isn't
/// already in the path.
pub fn write_wav(file_path: &Path, audio: &AudioData) -> Result<(), Error> {
    let wav_config = WavSpec {
        channels: 1,
        sample_rate: audio.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut file_path = file_path.to_owned();
    file_path.set_extension("wav");

    debug!(
        "Writing .wav file {:?} using config {:?}",
        file_path, wav_config
    );
    let mut writer = hound::WavWriter::create(file_path, wav_config)?;

    for &sample in &audio.data {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;

    Ok(())
}
