use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;

use hound::WavSpec;
use log::debug;
use ogg::{PacketWriteEndInfo, PacketWriter};
use rand::RngCore;

use crate::audio;
use crate::audio::AudioData;
use crate::error::Error;

#[derive(Default)]
pub enum AudioFileType {
    #[default]
    Wav,
    Opus,
}

impl From<&Path> for AudioFileType {
    fn from(value: &Path) -> Self {
        if let Some(Some(extension)) = value.extension().map(OsStr::to_str) {
            return match extension {
                "wav" => AudioFileType::Wav,
                "opus" => AudioFileType::Opus,
                _ => AudioFileType::default(),
            };
        }
        AudioFileType::default()
    }
}

/// Save audio data to a file determined by the file extension.
///
/// Returns an error if the file could not be written.
///
/// # Arguments
///
/// * `file_path`: Where to save the file.
/// * `audio`: The audio data to save.
///
/// # Examples
///
/// ```no_run
/// # use std::path::Path;
/// # use varys_audio::file::write_audio;
/// # use varys_audio::audio::AudioData;
/// let audio = AudioData {
///     data: vec![0_f32, 1_f32, 2_f32],
///     channels: 1,
///     sample_rate: 44100,
/// };
/// write_audio(Path::new("audio.wav"), &audio).unwrap();
/// write_audio(Path::new("audio.opus"), &audio).unwrap();
/// ```
pub fn write_audio(file_path: &Path, audio: &AudioData) -> Result<(), Error> {
    match AudioFileType::from(file_path) {
        AudioFileType::Wav => write_wav(file_path, audio),
        AudioFileType::Opus => write_opus(file_path, audio),
    }
}

/// Save audio data to a `.wav` file.
///
/// Returns an error if the file could not be written.
///
/// # Arguments
///
/// * `file_path`: Where to save the file. The extension `.wav` will be added if it isn't already in
/// the path.
/// * `audio`: The audio data to save.
///
/// # Examples
///
/// ```no_run
/// # use std::path::Path;
/// # use varys_audio::file::write_wav;
/// # use varys_audio::audio::AudioData;
/// let audio = AudioData {
///     data: vec![0_f32, 1_f32, 2_f32],
///     channels: 1,
///     sample_rate: 48000,
/// };
/// write_wav(Path::new("audio.wav"), &audio).unwrap();
/// ```
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

/// Save audio data encoded as Opus to an `.opus` file.
///
/// Returns an error if the file could not be written.
///
/// More information: https://datatracker.ietf.org/doc/html/rfc7845 and
/// https://datatracker.ietf.org/doc/html/rfc3533
///
/// # Arguments
///
/// * `file_path`: Where to save the file. The extension `.opus` will be added if it isn't already
/// in the path.
/// * `audio`: The audio data to save.
///
/// # Examples
///
/// ```no_run
/// # use std::path::Path;
/// # use varys_audio::file::write_opus;
/// # use varys_audio::audio::AudioData;
/// let audio = AudioData {
///     data: vec![0_f32, 1_f32, 2_f32],
///     channels: 1,
///     sample_rate: 48000,
/// };
/// write_opus(Path::new("audio.opus"), &audio).unwrap();
/// ```
pub fn write_opus(file_path: &Path, audio: &AudioData) -> Result<(), Error> {
    let mut file_path = file_path.to_owned();
    file_path.set_extension("opus");

    debug!("Writing .opus file {:?}", file_path);

    let (encoded_frames, padding, frame_size) = audio.encode_opus()?;
    let file = File::create(file_path)?;
    let mut writer = PacketWriter::new(file);

    let bitstream_serial = rand::thread_rng().next_u32();
    let frame_granule_size =
        (frame_size * audio::OPUS_SAMPLE_RATE) as u64 / audio.sample_rate as u64;
    let mut granule_position = 0;

    writer.write_packet(
        opus_id_header(audio, padding)?,
        bitstream_serial,
        PacketWriteEndInfo::EndPage,
        granule_position,
    )?;
    writer.write_packet(
        opus_comment_header()?,
        bitstream_serial,
        PacketWriteEndInfo::EndPage,
        granule_position,
    )?;
    let mut frames_iter = encoded_frames.iter().peekable();
    while let Some(frame) = frames_iter.next() {
        writer.write_packet(
            frame,
            bitstream_serial,
            if frames_iter.peek().is_some() {
                PacketWriteEndInfo::NormalPacket
            } else {
                PacketWriteEndInfo::EndStream
            },
            granule_position,
        )?;
        granule_position += frame_granule_size;
    }

    Ok(())
}

fn opus_id_header(audio: &AudioData, padding: u16) -> Result<Vec<u8>, Error> {
    // the identification header is structured as follows:
    //
    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |      'O'      |      'p'      |      'u'      |      's'      |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |      'H'      |      'e'      |      'a'      |      'd'      |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  Version = 1  | Channel Count |           Pre-skip            |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                     Input Sample Rate (Hz)                    |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |   Output Gain (Q7.8 in dB)    | Mapping Family|               |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+               :
    // |                                                               |
    // :               Optional Channel Mapping Table...               :
    // |                                                               |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    //
    // (see https://datatracker.ietf.org/doc/html/rfc7845#section-5.1)

    let mut header = Vec::with_capacity(19);
    header.extend(b"OpusHead");
    header.push(1); // opus version number
    header.push(audio.channels);
    header.extend(&padding.to_le_bytes()); // pre-skip (see https://datatracker.ietf.org/doc/html/rfc7845#section-4.2)
    header.extend(&audio.sample_rate.to_le_bytes()); // samples per second
    header.extend(&0_u16.to_le_bytes()); // output gain
    header.push(0); // mapping family

    Ok(header)
}

fn opus_comment_header() -> Result<Vec<u8>, Error> {
    // the comment header is structured as follows:
    //
    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |      'O'      |      'p'      |      'u'      |      's'      |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |      'T'      |      'a'      |      'g'      |      's'      |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                     Vendor String Length                      |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                                                               |
    // :                        Vendor String...                       :
    // |                                                               |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                   User Comment List Length                    |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                 User Comment #0 String Length                 |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                                                               |
    // :                   User Comment #0 String...                   :
    // |                                                               |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                 User Comment #1 String Length                 |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // :                                                               :
    //
    // (see https://datatracker.ietf.org/doc/html/rfc7845#section-5.2)

    let vendor = format!("varys {}", env!("CARGO_PKG_VERSION"));
    let mut header = Vec::new();
    header.extend(b"OpusTags");
    header.extend(&(vendor.len() as u32).to_le_bytes()); // vendor string length
    header.extend(vendor.bytes()); // vendor string
    header.extend(&0_u32.to_le_bytes()); // comment list length

    Ok(header)
}
