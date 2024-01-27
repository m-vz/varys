use std::cmp::min;

use audiopus::coder::Encoder;
use audiopus::{Application, Bitrate, Channels, SampleRate};
use log::{debug, trace};

use crate::error::Error;

const OPUS_FRAME_TIME: usize = 20; // ms (see https://datatracker.ietf.org/doc/html/rfc6716#section-2.1.4)
const OPUS_FRAME_RATE: usize = 1000 / OPUS_FRAME_TIME; // 1/s
pub const OPUS_SAMPLE_RATE: usize = 48000; // 1/s (see https://datatracker.ietf.org/doc/html/rfc7845#section-4)
/// How many silent samples to keep when trimming silence from the start and end of audio.
pub const TRIM_SILENCE_PADDING: usize = 4800; // 0.1s

/// Holds interleaved audio data for one or more channels.
pub struct AudioData {
    /// The audio data in interleaved format.
    /// With two channels, this looks like `[l0, r0, l1, r1, ...]`
    pub data: Vec<f32>,
    /// The amount of channels stored.
    pub channels: u8,
    /// The sample rate of the audio data.
    pub sample_rate: u32,
}

impl AudioData {
    /// Convert the audio data to mono by taking the average of all channels for each sample.
    ///
    /// Does nothing if this is already mono audio.
    ///
    /// # Examples
    ///
    /// If the audio is already mono, this is essentially a no-op:
    ///
    /// ```
    /// # use varys::listen::audio::AudioData;
    /// let mut audio = AudioData {
    ///     data: vec![0_f32, 1_f32, 2_f32],
    ///     channels: 1,
    ///     sample_rate: 44100,
    /// };
    /// assert_eq!(audio.convert_to_mono().data, vec![0_f32, 1_f32, 2_f32]);
    /// ```
    ///
    /// Otherwise, the channels are combined into their average:
    ///
    /// ```
    /// # use varys::listen::audio::AudioData;
    /// let mut audio = AudioData {
    ///     data: vec![0_f32, 1_f32, 2_f32, 3_f32],
    ///     channels: 2,
    ///     sample_rate: 44100,
    /// };
    /// assert_eq!(audio.convert_to_mono().data, vec![0.5_f32, 2.5_f32]);
    /// ```
    pub fn convert_to_mono(&mut self) -> &mut Self {
        if self.channels != 1 {
            debug!("Converting from {} channels to mono", self.channels);

            self.data = self
                .data
                .chunks_exact(self.channels as usize)
                .map(|x| x.iter().sum::<f32>() / self.channels as f32)
                .collect();
            self.channels = 1;
        }
        self
    }

    /// Downsample the audio data to a lower sample rate.
    ///
    /// Does nothing if the sample rate is the same as the current one.
    ///
    /// Returns an error if the new sample rate is not a divisor of the current sample rate.
    ///
    /// This uses the nearest-neighbour algorithm, which works well when downsampling by an
    /// integer factor.
    ///
    /// # Arguments
    ///
    /// * `sample_rate`: The new sample rate to downsample to. Must be a divisor of the current
    /// sample rate.
    ///
    /// # Examples
    ///
    /// This examples samples from 48kHz to 16kHz by a factor of 3. Therefore every third element is
    /// kept when creating the downsampled data.
    ///
    /// ```
    /// # use varys::listen::audio::AudioData;
    /// let mut audio = AudioData {
    ///     data: vec![0_f32, 1_f32, 2_f32, 3_f32, 4_f32],
    ///     channels: 1,
    ///     sample_rate: 48000,
    /// };
    /// assert_eq!(audio.downsample(16000).unwrap().data, vec![0_f32, 3_f32]);
    /// ```
    pub fn downsample(&mut self, sample_rate: u32) -> Result<&mut Self, Error> {
        if self.sample_rate % sample_rate != 0 {
            return Err(Error::NoDivisor);
        }

        if self.sample_rate == sample_rate {
            return Ok(self);
        }

        debug!("Resampling {}Hz to {}Hz", self.sample_rate, sample_rate);

        let sample_ratio = (self.sample_rate / sample_rate) as usize;
        let resampled_length = self.data.len() / sample_ratio + 1; // add 1 to make sure the array doesn't need to grow
        let mut resampled_data = Vec::with_capacity(resampled_length);
        self.data
            .chunks_exact(self.channels as usize)
            .step_by(sample_ratio)
            .for_each(|chunk| resampled_data.append(&mut chunk.to_vec()));
        self.data = resampled_data;
        self.sample_rate = sample_rate;

        Ok(self)
    }

    /// Trim silent parts of the audio from the start and the end.
    ///
    /// If there is no audio above the threshold, the data is cleared.
    ///
    /// # Arguments
    ///
    /// * `threshold`: Determines what samples are considered silent.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys::listen::audio::{AudioData, TRIM_SILENCE_PADDING};
    /// let mut content = vec![1_f32, 2_f32, 3_f32, 4_f32];
    ///
    /// // actual
    /// let mut silence = vec![0_f32; TRIM_SILENCE_PADDING * 2];
    /// let mut data = silence.clone();
    /// data.append(&mut content.clone());
    /// data.append(&mut silence);
    ///
    /// // expected
    /// let mut expected_silence = vec![0_f32; TRIM_SILENCE_PADDING];
    /// let mut expected_data = expected_silence.clone();
    /// expected_data.append(&mut content);
    /// expected_data.append(&mut expected_silence);
    ///
    /// let mut audio = AudioData {
    ///     data,
    ///     channels: 1,
    ///     sample_rate: 48000,
    /// };
    /// assert_eq!(audio.trim_silence(1_f32).data, expected_data);
    /// ```
    pub fn trim_silence(&mut self, threshold: f32) -> &mut Self {
        // find the index of the first sample that is above the threshold
        let from = self
            .data
            .iter()
            .enumerate()
            .find(|(_, &sample)| sample >= threshold)
            .map(|(i, _)| i);

        if let Some(first) = from {
            // there is at least one sample above the threshold
            // find the index of the last sample that is above the threshold
            // we can unwrap because we know there is at least one sample above the threshold
            let last = self
                .data
                .iter()
                .enumerate()
                .rev()
                .find(|(_, &sample)| sample >= threshold)
                .map(|(i, _)| i)
                .unwrap();
            // add padding
            let first = first.saturating_sub(TRIM_SILENCE_PADDING);
            let last = min(
                last.saturating_add(TRIM_SILENCE_PADDING),
                self.data.len() - 1,
            );
            // trim the data
            self.data = self.data[first..=last].to_vec();
        } else {
            // there are no samples above the threshold
            // clear the data
            self.data = Vec::new();
        }

        self
    }

    /// Encode the audio data into OPUS frames.
    ///
    /// Returns the OPUS frames, the size of the padding added to the start and the number of samples per frame.
    ///
    /// Returns an error if encoding failed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys::listen::audio::AudioData;
    /// let mut audio = AudioData {
    ///     data: vec![0_f32, 1_f32, 2_f32, 3_f32, 4_f32],
    ///     channels: 1,
    ///     sample_rate: 48000,
    /// };
    /// audio.encode_opus().unwrap();
    /// ```
    pub fn encode_opus(&self) -> Result<(Vec<Vec<u8>>, u16, usize), Error> {
        if self.data.is_empty() {
            return Ok((vec![], 0, 0));
        }

        let sample_rate = i32::try_from(self.sample_rate).map_err(|_| Error::OutOfRange)?;
        let channels: i32 = self.channels.into();
        let mut encoder = Encoder::new(
            SampleRate::try_from(sample_rate)?,
            Channels::try_from(channels)?,
            Application::Voip,
        )?;
        encoder.set_bitrate(Bitrate::BitsPerSecond(24000))?;

        let frame_size = (sample_rate * channels) as usize / OPUS_FRAME_RATE;
        let mut encoded_frames = Vec::new();

        let padding = u16::try_from(encoder.lookahead()?).map_err(|_| Error::OutOfRange)?;
        let mut padded_data = vec![0.0; padding as usize];
        padded_data.extend(self.data.iter());

        for start in (0..padded_data.len()).step_by(frame_size) {
            let end = start + frame_size;
            if end >= padded_data.len() {
                // drop the last packet if it doesn't fit – opus only supports specific frame sizes
                break;
            }
            let mut encoded_frame = vec![0; frame_size];

            loop {
                match encoder.encode_float(&padded_data[start..end], &mut encoded_frame) {
                    Ok(length) => {
                        encoded_frame.truncate(length);
                        encoded_frames.push(encoded_frame);
                        break Ok(());
                    }
                    Err(audiopus::Error::Opus(audiopus::ErrorCode::BufferTooSmall)) => {
                        trace!("Buffer size too small, doubling it");
                        encoded_frame.reserve(encoded_frame.capacity());
                    }
                    Err(e) => break Err(Error::from(e)),
                }
            }?;
        }

        Ok((
            encoded_frames,
            padding,
            sample_rate as usize / OPUS_FRAME_RATE,
        ))
    }

    /// Get the duration of the audio in milliseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys::listen::audio::AudioData;
    /// let sample_rate = 48000;
    /// let seconds = 100_i32;
    /// let mut audio = AudioData {
    ///     data: vec![0_f32; seconds as usize * sample_rate],
    ///     channels: 1,
    ///     sample_rate: sample_rate as u32,
    /// };
    /// assert_eq!(audio.duration(), seconds * 1000);
    /// ```
    pub fn duration(&self) -> i32 {
        (self.data.len() as i64 * 1000 / self.sample_rate as i64 / self.channels as i64) as i32
    }
}

impl AsRef<[f32]> for AudioData {
    fn as_ref(&self) -> &[f32] {
        &self.data
    }
}

impl From<AudioData> for Vec<f32> {
    fn from(value: AudioData) -> Self {
        value.data
    }
}
