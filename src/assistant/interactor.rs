use std::path::Path;
use std::time::Duration;

use chrono::Utc;
use log::info;

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
}

impl Interactor {
    pub fn with(
        interface: String,
        voice: String,
        sensitivity: f32,
        model: Model,
    ) -> Result<Self, Error> {
        Ok(Self {
            listener: Listener::new()?,
            sniffer: Sniffer::from(sniff::device_by_name(interface.as_str())?),
            speaker: Speaker::with_voice(voice.as_str())?,
            sensitivity,
            recogniser: Recogniser::with_model(model)?,
        })
    }

    pub async fn start(&mut self, queries: Vec<String>) -> Result<(), Error> {
        let pool = database::connect().await?;
        let mut session = Session::new(&pool).await?;

        for query in queries {
            info!("Saying {}", query);

            // prepare the interaction
            let mut interaction = session.new_interaction(&pool).await?;

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
            info!("{}", self.recogniser.recognise(&mut audio)?);

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
