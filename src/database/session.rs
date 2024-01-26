use chrono::{DateTime, Utc};
use clap::crate_version;
use sqlx::{FromRow, PgPool};

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
    /// When this session was started.
    pub started: DateTime<Utc>,
    /// When this session was ended.
    #[sqlx(default)]
    pub ended: Option<DateTime<Utc>>,
}

impl Session {
    /// Create a new session in the database.
    ///
    /// # Arguments
    ///
    /// * `pool`: The connection pool to use.
    pub async fn create(pool: &PgPool, config: &InteractorConfig) -> Result<Self, Error> {
        let started = Utc::now();
        let version = crate_version!().to_string();
        let interactor_config_id = config.get_or_create(pool).await?;
        let id = sqlx::query!(
            "INSERT INTO session (started, version, interactor_config_id) VALUES ($1, $2, $3) RETURNING id",
            started,
            version,
            interactor_config_id,
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(Session {
            id,
            version,
            interactor_config_id,
            data_dir: None,
            started,
            ended: None,
        })
    }

    /// Get an session from the database.
    ///
    /// # Arguments
    ///
    /// * `pool`: The connection pool to use.
    /// * `id`: The id of the session.
    pub async fn get(id: i32, pool: &PgPool) -> Result<Option<Self>, Error> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM session WHERE id = $1", id)
                .fetch_optional(pool)
                .await?,
        )
    }

    /// Update all values of a session in the database.
    ///
    /// # Arguments
    ///
    /// * `pool`: The connection pool to use.
    pub async fn update(&mut self, pool: &PgPool) -> Result<&mut Self, Error> {
        sqlx::query!(
            "UPDATE session SET (version, interactor_config_id, data_dir, started, ended) = ($1, $2, $3, $4, $5) WHERE id = $6",
            self.version,
            self.interactor_config_id,
            self.data_dir,
            self.started,
            self.ended,
            self.id
        )
            .execute(pool)
            .await?;

        Ok(self)
    }

    /// Mark a session as completed by setting its end time.
    ///
    /// # Arguments
    ///
    /// * `pool`: The connection pool to use.
    pub async fn complete(&mut self, pool: &PgPool) -> Result<&mut Self, Error> {
        self.ended = Some(Utc::now());
        self.update(pool).await?;

        Ok(self)
    }

    /// Get the `InteractorConfig` for this session.
    ///
    /// # Arguments
    ///
    /// * `pool`: The connection pool to use.
    pub async fn config(&self, pool: &PgPool) -> Result<Option<InteractorConfig>, Error> {
        InteractorConfig::get(self.interactor_config_id, pool).await
    }
}