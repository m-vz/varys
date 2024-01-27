use sqlx::{FromRow, PgPool};

use crate::database;
use crate::error::Error;

/// The representation of a interactor configuration in the database.
///
/// Each config is uniquely represented in the database.
#[derive(FromRow, Debug)]
pub struct InteractorConfig {
    pub interface: String,
    pub voice: String,
    pub sensitivity: String,
    pub model: String,
}

impl InteractorConfig {
    /// Get an interactor config from the database or create it if it doesn't exist yet.
    ///
    /// Every combination of interface, voice, sensitivity and model is uniquely represented in the
    /// database, so we cannot just create a new config if the same one already exists.
    ///
    /// # Arguments
    ///
    /// * `pool`: The connection pool to use.
    pub async fn get_or_create(&self, pool: &PgPool) -> Result<i32, Error> {
        // first, try to find an existing config with the same values ...
        let query = sqlx::query!(
            "SELECT id FROM interactor_config WHERE interface = $1 AND voice = $2 AND sensitivity = $3 AND model = $4",
            self.interface,
            self.voice,
            self.sensitivity,
            self.model,
        );

        database::log_query(&query);
        if let Some(result) = query.fetch_optional(pool).await? {
            return Ok(result.id);
        }

        // ... otherwise, create a new one
        let query = sqlx::query!(
                "INSERT INTO interactor_config (interface, voice, sensitivity, model) VALUES ($1, $2, $3, $4) RETURNING id",
                self.interface,
                self.voice,
                self.sensitivity,
                self.model,
            );

        database::log_query(&query);
        Ok(query.fetch_one(pool).await?.id)
    }

    /// Get an interactor config from the database.
    ///
    /// # Arguments
    ///
    /// * `pool`: The connection pool to use.
    /// * `id`: The id of the config.
    pub async fn get(id: i32, pool: &PgPool) -> Result<Option<Self>, Error> {
        let query = sqlx::query!("SELECT * FROM interactor_config WHERE id = $1", id);

        database::log_query(&query);
        if let Some(result) = query.fetch_optional(pool).await? {
            Ok(Some(InteractorConfig {
                interface: result.interface,
                voice: result.voice,
                sensitivity: result.sensitivity,
                model: result.model,
            }))
        } else {
            Ok(None)
        }
    }
}
