use log::debug;

use crate::error::Error;

/// Holds interleaved audio data for one or more channels.
pub struct AudioData {
    /// The audio data in interleaved format.
    /// With two channels, this looks like `[l0, r0, l1, r1, ...]`
    pub data: Vec<f32>,
    /// The amount of channels stored.
    pub channels: u16,
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
