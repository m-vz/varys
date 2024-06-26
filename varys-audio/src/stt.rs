use log::{debug, info, trace, warn};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::audio::AudioData;
use crate::error::Error;

pub mod transcribe;
pub mod transcriber;

/// Wraps the whisper API.
pub struct Recogniser {
    context: WhisperContext,
}

impl Recogniser {
    /// This sample rate is expected by whisper, so all audio data has to be resampled to this.
    pub const SAMPLE_RATE: u32 = 16_000;

    /// Create a new recogniser that uses the model stored at the given file path.
    ///
    /// Returns an error if the model could not be loaded or does not have proper `ggml` format.
    ///
    /// # Arguments
    ///
    /// * `model_path`: The path to the whisper model to use. The model must be in `ggml` format.
    /// (See [here](https://github.com/ggerganov/whisper.cpp/tree/master/models) for more
    /// information.)
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys_audio::stt::{Model, MODEL_LARGE, Recogniser};
    /// # let path = format!("../{}", MODEL_LARGE);
    /// let recogniser = Recogniser::with_model_path(&path).unwrap();
    /// ```
    pub fn with_model_path(model_path: &str) -> Result<Recogniser, Error> {
        let mut params = WhisperContextParameters::default();
        params.use_gpu(true);

        info!("Using model: {model_path}");

        Ok(Recogniser {
            context: WhisperContext::new_with_params(model_path, params)?,
        })
    }

    /// Convert speech in the given audio data to text.
    ///
    /// Forwards any errors that whisper returns.
    ///
    /// This method first preprocesses the audio to mono and resamples it to a sample rate of
    /// [`Recogniser::SAMPLE_RATE`].
    ///
    /// # Arguments
    ///
    /// * `audio`: The audio to recognise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys_audio::audio::AudioData;
    /// # use varys_audio::stt::{Model, MODEL_LARGE, Recogniser};
    /// # let path = format!("../{}", MODEL_LARGE);
    /// let mut audio = AudioData {
    ///     data: vec![0_f32],
    ///     channels: 1,
    ///     sample_rate: 16000,
    /// };
    /// let recogniser = Recogniser::with_model_path(&path).unwrap();
    /// let _ = recogniser.recognise(&mut audio);
    /// ```
    pub fn recognise(&self, audio: &mut AudioData) -> Result<String, Error> {
        if audio.duration_s() < 1.0 {
            warn!("Whisper cannot recognise audio shorter than one second");

            return Err(Error::RecordingTooShort);
        }

        debug!("Recognising {:.2} seconds of audio...", audio.duration_s());

        Recogniser::preprocess(audio)?;

        let mut state = self.context.create_state()?;
        let mut full_text = String::new();

        state.full(self.get_params(), &audio.data)?;

        let segment_count = state.full_n_segments()?;
        for i in 0..segment_count {
            let segment = state.full_get_segment_text(i)?;
            full_text.push_str(&segment);
            let timestamps = (state.full_get_segment_t0(i)?, state.full_get_segment_t1(i)?);
            trace!(
                "Recognised segment [{} - {}]: {}",
                timestamps.0,
                timestamps.1,
                segment
            );
        }

        debug!("Recognised: {}", full_text);

        Ok(full_text)
    }

    fn preprocess(audio: &mut AudioData) -> Result<(), Error> {
        debug!("Preprocessing audio for recognition...");

        audio
            .convert_to_mono()
            .downsample(Recogniser::SAMPLE_RATE)?;

        Ok(())
    }

    fn get_params(&self) -> FullParams {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_print_special(false);
        params.set_suppress_non_speech_tokens(true);
        params.set_suppress_blank(true);
        params
    }
}
