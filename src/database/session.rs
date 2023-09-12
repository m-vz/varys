use chrono::{DateTime, Utc};
use clap::crate_version;
use sqlx::{FromRow, PgPool};

use crate::error::Error;

/// The representation of a session in the database.
///
/// A session can have one or more [`Interaction`]s.
#[derive(FromRow, Debug)]
pub struct Session {
    pub id: i32,
    pub version: String,
    pub started: DateTime<Utc>,
    #[sqlx(default)]
    pub ended: Option<DateTime<Utc>>,
}

impl Session {
    /// Create a new session in the database
    ///
    /// # Arguments
    ///
    /// * `pool`: The database pool to use
    pub async fn create(pool: &PgPool) -> Result<Self, Error> {
        let started = Utc::now();
        let version = crate_version!().to_string();
        let id = sqlx::query!(
            "INSERT INTO session (started, version) VALUES ($1, $2) RETURNING id",
            started,
            version
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(Session {
            id,
            version,
            started,
            ended: None,
        })
    }

    /// Get an session from the database
    ///
    /// # Arguments
    ///
    /// * `pool`: The database pool to use
    /// * `id`: The id of the session
    pub async fn get(id: i32, pool: &PgPool) -> Result<Option<Self>, Error> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM session WHERE id = $1", id)
                .fetch_optional(pool)
                .await?,
        )
    }

    /// Mark an session as completed by setting its end time
    ///
    /// # Arguments
    ///
    /// * `pool`: The database pool to use
    pub async fn complete(&mut self, pool: &PgPool) -> Result<(), Error> {
        let ended = Utc::now();
        sqlx::query!(
            "UPDATE session SET ended = $1 WHERE id = $2",
            ended,
            self.id
        )
        .execute(pool)
        .await?;
        self.ended = Some(ended);

        Ok(())
    }
}
