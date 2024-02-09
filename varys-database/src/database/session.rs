use std::fmt::{Display, Formatter};

use chrono::{DateTime, Utc};
use log::info;
use sqlx::FromRow;

use crate::connection::DatabaseConnection;
use crate::database;
use crate::database::interaction::Interaction;
use crate::database::interactor_config::InteractorConfig;
use crate::error::Error;

/// The representation of a session in the database.
///
/// A session can have one or more [`Interaction`]s.
#[derive(FromRow, Debug)]
pub struct Session {
    pub id: i32,
    /// What version of varys this session was run on.
    pub version: String,
    interactor_config_id: i32,
    /// The directory where the session data is stored.
    pub data_dir: Option<String>,
    /// The MAC address of the assistant.
    pub assistant_mac: String,
    /// When this session was started.
    pub started: DateTime<Utc>,
    /// When this session was ended.
    ///
    /// If this is `None`, the interaction is still running or was aborted.
    pub ended: Option<DateTime<Utc>>,
}

impl Session {
    /// Create a new session in the database.
    ///
    /// # Arguments
    ///
    /// * `connection`: The connection to use.
    pub async fn create(
        connection: &DatabaseConnection,
        config: &InteractorConfig,
        version: String,
        assistant_mac: String,
    ) -> Result<Self, Error> {
        let started = Utc::now();
        let interactor_config_id = config.get_or_create(connection).await?;
        let query = sqlx::query!(
            "INSERT INTO session (started, version, assistant_mac, interactor_config_id) VALUES ($1, $2, $3, $4) RETURNING id",
            started,
            version,
            assistant_mac,
            interactor_config_id,
        );

        database::log_query(&query);
        let id = query.fetch_one(&connection.pool).await?.id;

        Ok(Session {
            id,
            version,
            interactor_config_id,
            data_dir: None,
            assistant_mac,
            started,
            ended: None,
        })
    }

    /// Get a session from the database.
    ///
    /// # Arguments
    ///
    /// * `connection`: The connection to use.
    /// * `id`: The id of the session.
    pub async fn get(id: i32, connection: &DatabaseConnection) -> Result<Option<Self>, Error> {
        let query = sqlx::query_as!(Self, "SELECT * FROM session WHERE id = $1", id);

        database::log_query(&query);
        Ok(query.fetch_optional(&connection.pool).await?)
    }

    /// Get all sessions from the database.
    ///
    /// # Arguments
    ///
    /// * `connection`: The connection to use.
    pub async fn get_all(connection: &DatabaseConnection) -> Result<Vec<Self>, Error> {
        let query = sqlx::query_as!(Self, "SELECT * FROM session");

        database::log_query(&query);
        Ok(query.fetch_all(&connection.pool).await?)
    }

    /// Update all values of a session in the database.
    ///
    /// # Arguments
    ///
    /// * `connection`: The connection to use.
    pub async fn update(&mut self, connection: &DatabaseConnection) -> Result<&mut Self, Error> {
        let query = sqlx::query!(
            "UPDATE session SET (version, assistant_mac, interactor_config_id, data_dir, started, ended) = ($1, $2, $3, $4, $5, $6) WHERE id = $7",
            self.version,
            self.assistant_mac,
            self.interactor_config_id,
            self.data_dir,
            self.started,
            self.ended,
            self.id
        );

        database::log_query(&query);
        query.execute(&connection.pool).await?;

        Ok(self)
    }

    /// Mark a session as completed by setting its end time.
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

    /// Get the `InteractorConfig` for this session.
    ///
    /// # Arguments
    ///
    /// * `connection`: The connection to use.
    pub async fn config(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<Option<InteractorConfig>, Error> {
        InteractorConfig::get(connection, self.interactor_config_id).await
    }

    /// Get all interactions for this session.
    ///
    /// # Arguments
    ///
    /// * `connection`: The connection to use.
    pub async fn interactions(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<Vec<Interaction>, Error> {
        Interaction::get_session(connection, self.id).await
    }
}

impl Display for Session {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Session {} (started {})", self.id, self.started)
    }
}
