use std::path::Path;
use std::time::Duration;

use chrono::Utc;
use log::info;

use crate::database::interaction::Interaction;
use crate::database::interactor_config::InteractorConfig;
use crate::database::session::Session;
use crate::error::Error;
use crate::listen::Listener;
use crate::recognise::{Model, Recogniser};
use crate::sniff::Sniffer;
use crate::speak::Speaker;
use crate::{database, file, sniff};

pub struct Interactor {
    listener: Listener,
    sniffer: Sniffer,
    speaker: Speaker,
    sensitivity: f32,
    recogniser: Recogniser,
    config: InteractorConfig,
}

impl Interactor {
    /// Create an interactor and all its components.
    ///
    /// This will create a [`Listener`], a [`Sniffer`], a [`Speaker`] and a [`Recogniser`].
    ///
    /// # Arguments
    ///
    /// * `interface`: The interface to create the sniffer on.
    /// * `voice`: The voice to use for the speaker.
    /// * `sensitivity`: The sensitivity of the listener.
    /// * `model`: The model to use for the recogniser.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys::assistant::interactor::Interactor;
    /// # use varys::recognise::Model;
    /// let mut interactor =
    ///     Interactor::with("ap1".to_string(), "Zoe".to_string(), 0.01, Model::Large).unwrap();
    /// ```
    pub fn with(
        interface: String,
        voice: String,
        sensitivity: f32,
        model: Model,
    ) -> Result<Self, Error> {
        let config = InteractorConfig {
            interface: interface.clone(),
            voice: voice.clone(),
            sensitivity: sensitivity.to_string(),
            model: model.to_string(),
        };

        Ok(Self {
            listener: Listener::new()?,
            sniffer: Sniffer::from(sniff::device_by_name(interface.as_str())?),
            speaker: Speaker::with_voice(voice.as_str())?,
            sensitivity,
            recogniser: Recogniser::with_model(model)?,
            config,
        })
    }

    /// Start a new session of interactions.
    ///
    /// # Arguments
    ///
    /// * `queries`: The queries to ask during this session.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use varys::assistant::interactor::Interactor;
    /// # use varys::recognise::Model;
    /// let mut interactor =
    ///     Interactor::with("ap1".to_string(), "Zoe".to_string(), 0.01, Model::Large).unwrap();
    /// let queries = vec!["How are you?".to_string(), "What is your name?".to_string()];
    /// # tokio::runtime::Builder::new_current_thread()
    /// #     .enable_all()
    /// #     .build()
    /// #     .unwrap()
    /// #     .block_on(async {
    /// interactor.start(queries).await.unwrap();
    /// #     })
    /// ```
    pub async fn start(&mut self, queries: Vec<String>) -> Result<(), Error> {
        let pool = database::connect().await?;
        let mut session = Session::create(&pool, &self.config).await?;

        for query in queries {
            info!("Saying {}", query);

            // prepare the interaction
            let mut interaction = Interaction::create(&pool, &session, query.as_str()).await?;

            // start the sniffer
            let file_path = format!("query-{}.pcap", Utc::now().format("%Y-%m-%d-%H-%M-%S-%f"));
            let file_path = Path::new(file_path.as_str());
            let sniffer_instance = self.sniffer.start(file_path)?;

            // say the query and record the response
            self.speaker.say(&query, true)?;
            let mut audio = self
                .listener
                .record_until_silent(Duration::from_secs(2), self.sensitivity)?;

            // recognise the response
            let response = self.recogniser.recognise(&mut audio)?;
            interaction.add_response(&pool, response.as_str()).await?;
            info!("{response}");

            // finish the sniffer
            info!("{}", sniffer_instance.stop()?);
            file::compress_gzip(file_path, false)?;

            // finish the interaction
            interaction.complete(&pool).await?;
        }

        session.complete(&pool).await?;

        Ok(())
    }
}
