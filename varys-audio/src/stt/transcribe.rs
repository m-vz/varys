pub trait Transcribe: Sync + Send {
    /// This method will be called after successfully transcribing.
    ///
    /// # Arguments
    ///
    /// * `text`: The text that was transcribed.
    fn transcribed(&mut self, text: String);
}

impl Transcribe for Option<String> {
    fn transcribed(&mut self, text: String) {
        *self = Some(text);
    }
}
