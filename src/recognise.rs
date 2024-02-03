use std::fmt::Display;

use log::{debug, info, trace, warn};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::error::Error;
use crate::listen::audio::AudioData;

pub const MODEL_LARGE: &str = "data/models/ggml-model-whisper-large-q5_0.bin";
pub const MODEL_MEDIUM_EN: &str = "data/models/ggml-model-whisper-medium.en-q5_0.bin";

#[derive(Default, Copy, Clone)]
pub enum Model {
    #[default]
    Large,
    Medium,
}

impl From<String> for Model {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "large" => Model::Large,
            "medium" => Model::Medium,
            _ => Model::default(),
        }
    }
}

impl Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Model::Large => write!(f, "Large"),
            Model::Medium => write!(f, "Medium"),
        }
    }
}

/// Wraps the whisper API.
pub struct Recogniser {
    context: WhisperContext,
}

impl Recogniser {
    /// This sample rate is expected by whisper, so all audio data has to be resampled to this.
    pub const SAMPLE_RATE: u32 = 16_000;

    /// Create a new recogniser that uses one of the supplied models.
    ///
    /// Returns an error if the model could not be loaded or does not have proper `ggml` format.
    ///
    /// # Arguments
    ///
    /// * `model`: The model to use for this recogniser.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys::recognise::{Model, Recogniser};
    /// let recogniser = Recogniser::with_model(Model::Large).unwrap();
    /// ```
    pub fn with_model(model: Model) -> Result<Recogniser, Error> {
        let model_path = match model {
            Model::Large => MODEL_LARGE,
            Model::Medium => MODEL_MEDIUM_EN,
        };

        Recogniser::with_model_path(model_path)
    }

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
    /// # use varys::recognise::{Model, Recogniser};
    /// let recogniser = Recogniser::with_model_path(varys::recognise::MODEL_LARGE).unwrap();
    /// ```
    pub fn with_model_path(model_path: &str) -> Result<Recogniser, Error> {
        let mut params = WhisperContextParameters::default();
        params.use_gpu(true);

        info!("Using model: {}", model_path);

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
    /// # use varys::listen::audio::AudioData;
    /// use varys::recognise::{Model, Recogniser};
    /// let mut audio = AudioData {
    ///     data: vec![0_f32],
    ///     channels: 1,
    ///     sample_rate: 16000,
    /// };
    /// let recogniser =
    ///     Recogniser::with_model(Model::Medium).unwrap();
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
