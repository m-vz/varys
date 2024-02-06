use std::fmt::Display;

use chrono::{DateTime, Utc};
use log::info;
use sqlx::FromRow;

use crate::connection::DatabaseConnection;
use crate::database;
use crate::database::session::Session;
use crate::error::Error;

/// The representation of an interaction in the database.
///
/// Each interaction belongs to a [`Session`].
#[derive(FromRow, Debug)]
pub struct Interaction {
    /// Interaction ids are sequenced.
    pub id: i32,
    /// The id of the session this interaction was held in.
    ///
    /// Session ids are sequenced.
    pub session_id: i32,
    /// The query that was asked for this interaction.
    pub query: String,
    /// The category of the query.
    pub query_category: String,
    /// The duration of the query in milliseconds.
    ///
    /// If this is `None`, the interaction is still running or was aborted.
    pub query_duration: Option<i32>,
    /// The file with the recorded query.
    ///
    /// Stored inside the session `data_dir`.
    pub query_file: Option<String>,
    /// The recorded response from the voice assistant.
    ///
    /// Currently, short responses are sometimes not recognised accurately. Watch `response_duration`
    /// for short times if the response is missing.
    ///
    /// If this is `None`, the interaction is still running or was aborted.
    pub response: Option<String>,
    /// The duration of the response in milliseconds.
    ///
    /// If this is `None`, the interaction is still running or was aborted.
    pub response_duration: Option<i32>,
    /// The file with the recorded response.
    ///
    /// Stored inside the session `data_dir`.
    pub response_file: Option<String>,
    /// The file with the captured traffic.
    ///
    /// Stored inside the session `data_dir`.
    pub capture_file: Option<String>,
    /// When this interaction was started.
    pub started: DateTime<Utc>,
    /// When this interaction was ended.
    ///
    /// If this is `None`, the interaction is still running or was aborted.
    pub ended: Option<DateTime<Utc>>,
}

impl Interaction {
    /// Create a new interaction in the database.
    ///
    /// # Arguments
    ///
    /// * `connection`: The connection to use.
    /// * `session`: The session to associate the interaction with.
    pub async fn create(
        connection: &DatabaseConnection,
        session: &Session,
        text: &str,
        category: &str,
    ) -> Result<Self, Error> {
        let started = Utc::now();
        let query = sqlx::query!(
            "INSERT INTO interaction (started, session_id, query, query_category) VALUES ($1, $2, $3, $4) RETURNING id",
            started,
            session.id,
            text,
            category,
        );

        database::log_query(&query);
        let id = query.fetch_one(&connection.pool).await?.id;

        Ok(Interaction {
            id,
            session_id: session.id,
            query: text.to_string(),
            query_category: category.to_string(),
            query_duration: None,
            query_file: None,
            response: None,
            response_duration: None,
            response_file: None,
            capture_file: None,
            started,
            ended: None,
        })
    }

    /// Get an interaction from the database.
    ///
    /// # Arguments
    ///
    /// * `connection`: The connection to use.
    /// * `id`: The id of the interaction.
    pub async fn get(connection: &DatabaseConnection, id: i32) -> Result<Option<Self>, Error> {
        let query = sqlx::query_as!(Self, "SELECT * FROM interaction WHERE id = $1", id);

        database::log_query(&query);
        Ok(query.fetch_optional(&connection.pool).await?)
    }

    /// Update all values of an interaction in the database.
    ///
    /// # Arguments
    ///
    /// * `connection`: The connection to use.
    pub async fn update(&mut self, connection: &DatabaseConnection) -> Result<&mut Self, Error> {
        let query = sqlx::query!(
            "UPDATE interaction SET (session_id, query, query_category, query_duration, query_file, response, response_duration, response_file, capture_file, started, ended) = ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) WHERE id = $12",
            self.session_id,
            self.query,
            self.query_category,
            self.query_duration,
            self.query_file,
            self.response,
            self.response_duration,
            self.response_file,
            self.capture_file,
            self.started,
            self.ended,
            self.id
        );

        database::log_query(&query);
        query.execute(&connection.pool).await?;

        Ok(self)
    }

    /// Mark an interaction as completed by setting its end time.
    ///
    /// # Arguments
    ///
    /// * `connection`: The connection to use.
    pub async fn complete(&mut self, connection: &DatabaseConnection) -> Result<&mut Self, Error> {
        self.ended = Some(Utc::now());
        self.update(connection).await?;

        info!("Completed {self} at {}", Utc::now());

        Ok(self)
    }
}

impl Display for Interaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Interaction {} ({})", self.id, self.query)
    }
}
