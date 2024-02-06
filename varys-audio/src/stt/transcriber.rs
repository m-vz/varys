use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use log::{debug, error};

use crate::audio::AudioData;
use crate::error::Error;
use crate::stt::transcribe::Transcribe;
use crate::stt::Recogniser;

/// A transcriber that can run in the background to transcribe audio.
///
/// This is always created together with a [`TranscriberHandle`], which is used to communicate with the transcriber once
/// it has started.
pub struct Transcriber<T: Transcribe> {
    recogniser: Recogniser,
    audio_receiver: Receiver<(T, AudioData)>,
    result_sender: Sender<T>,
    stop_receiver: Receiver<()>,
}

impl<T: Transcribe> Transcriber<T> {
    /// Create a new transcriber and a [`TranscriberHandle`] to go with it.
    ///
    /// # Arguments
    ///
    /// * `recogniser`: The recogniser to use for audio transcription.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys_audio::stt::{ MODEL_LARGE, Recogniser};
    /// # use varys_audio::stt::transcribe::Transcribe;
    /// # use varys_audio::stt::transcriber::Transcriber;
    /// # let path = format!("../{}", MODEL_LARGE);
    /// let (transcriber, transcriber_handle): (Transcriber<dyn Transcribe>, _) =
    ///     Transcriber::new(Recogniser::with_model_path(&path).unwrap());
    /// ```
    pub fn new(recogniser: Recogniser) -> (Self, TranscriberHandle<T>) {
        let (audio_sender, audio_receiver) = std::sync::mpsc::channel();
        let (result_sender, result_receiver) = std::sync::mpsc::channel();
        let (stop_sender, stop_receiver) = std::sync::mpsc::channel();

        (
            Self {
                recogniser,
                audio_receiver,
                result_sender,
                stop_receiver,
            },
            TranscriberHandle::Sender(TranscriberSender {
                audio_sender,
                result_receiver,
                stop_sender,
            }),
        )
    }

    /// Start the transcriber loop.
    ///
    /// This should be called inside a new thread and will until it is stopped or encounters a transcription error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::thread;
    /// # use varys_audio::error::Error;
    /// # use varys_audio::stt::{Model, Recogniser};
    /// # use varys_audio::stt::transcribe::Transcribe;
    /// # use varys_audio::stt::transcriber::Transcriber;
    /// let (transcriber, transcriber_handle): (Transcriber<dyn Transcribe>, _) =
    ///     Transcriber::new(Recogniser::with_model(Model::default()).unwrap());
    /// let join_handle = thread::spawn(move || transcriber.start());
    /// ```
    pub fn start(&self) -> Result<(), Error> {
        debug!("Started transcriber");

        loop {
            if let Ok(()) = self.stop_receiver.try_recv() {
                debug!("Stopped transcriber");

                return Ok(());
            }

            match self.audio_receiver.try_recv() {
                Ok((mut transcribe, mut audio)) => {
                    match self.recogniser.recognise(&mut audio) {
                        Ok(text) => {
                            transcribe.transcribed(text);
                        }
                        Err(error) => {
                            error!("Failed to recognise response to: {error}");
                        }
                    }

                    self.result_sender
                        .send(transcribe)
                        .map_err(|_| Error::TranscriberStopped)?;
                }
                Err(TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(100));
                }
                Err(TryRecvError::Disconnected) => {
                    return Err(Error::TranscriberStopped);
                }
            }
        }
    }
}

/// A TranscriberHandle is used to communicate with a [`Transcriber`] once it has started.
///
/// To create a TranscriberHandle, use [`Transcriber::new`].
pub enum TranscriberHandle<T: Transcribe> {
    Sender(TranscriberSender<T>),
    Receiver(TranscriberReceiver<T>),
}

pub struct TranscriberSender<T: Transcribe> {
    audio_sender: Sender<(T, AudioData)>,
    result_receiver: Receiver<T>,
    stop_sender: Sender<()>,
}

impl<T: Transcribe> TranscriberSender<T> {
    /// Send audio to the [`Transcriber`] for transcription.
    ///
    /// This does not wait for the [`Transcriber`] to finish.
    ///
    /// # Arguments
    ///
    /// * `transcribe`: A [`Transcribe`] that will be updated once transcription is complete.
    /// * `audio`: The [`AudioData`] to transcribe.
    ///
    /// Returns a [`TranscriberReceiver`] that can be used to later receive the transcription result.
    pub fn transcribe(self, transcribe: T, audio: AudioData) -> TranscriberReceiver<T> {
        debug!("Sending audio to transcription thread...");

        self.audio_sender.send((transcribe, audio)).unwrap();

        TranscriberReceiver {
            audio_sender: self.audio_sender,
            result_receiver: self.result_receiver,
            stop_sender: self.stop_sender,
        }
    }

    /// Stop the [`Transcriber`] and consume this handle to it.
    pub fn stop(self) {
        debug!("Stopping transcriber...");

        let _ = self.stop_sender.send(());
    }
}

pub struct TranscriberReceiver<T: Transcribe> {
    audio_sender: Sender<(T, AudioData)>,
    result_receiver: Receiver<T>,
    stop_sender: Sender<()>,
}

impl<T: Transcribe> TranscriberReceiver<T> {
    /// Try to receive audio from the [`Transcriber`].
    ///
    /// This will block the current thread until the transcriber has finished.
    ///
    /// Returns a [`TranscriberSender`] that can be used to start another transcription and the transcribed
    /// [`Transcribe`].
    pub fn receive(self) -> (TranscriberSender<T>, Result<T, Error>) {
        debug!("Receiving audio from transcription thread...");

        let result = self
            .result_receiver
            .recv()
            .map_err(|_| Error::TranscriberStopped);

        (
            TranscriberSender {
                audio_sender: self.audio_sender,
                result_receiver: self.result_receiver,
                stop_sender: self.stop_sender,
            },
            result,
        )
    }
}

impl<T: Transcribe> From<TranscriberSender<T>> for TranscriberHandle<T> {
    fn from(sender: TranscriberSender<T>) -> Self {
        TranscriberHandle::Sender(sender)
    }
}

impl<T: Transcribe> From<TranscriberReceiver<T>> for TranscriberHandle<T> {
    fn from(receiver: TranscriberReceiver<T>) -> Self {
        TranscriberHandle::Receiver(receiver)
    }
}
