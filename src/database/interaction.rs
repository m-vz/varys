use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::database::session::Session;
use crate::error::Error;

/// The representation of an interaction in the database.
/// 
/// Each interaction belongs to a [`Session`].
#[derive(FromRow, Debug)]
pub struct Interaction {
    pub id: i32,
    pub started: DateTime<Utc>,
    #[sqlx(default)]
    pub ended: Option<DateTime<Utc>>,
    pub session_id: i32,
}

impl Interaction {
    /// Create a new interaction in the database
    ///
    /// # Arguments
    ///
    /// * `pool`: The database pool to use
    /// * `session`: The session to associate the interaction with
    pub async fn create(pool: &PgPool, session: &Session) -> Result<Self, Error> {
        let started = Utc::now();
        let id = sqlx::query!(
            "INSERT INTO interaction (started, session_id) VALUES ($1, $2) RETURNING id",
            started,
            session.id
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(Interaction {
            id,
            started,
            ended: None,
            session_id: session.id,
        })
    }

    /// Get an interaction from the database
    ///
    /// # Arguments
    ///
    /// * `pool`: The database pool to use
    /// * `id`: The id of the interaction
    pub async fn get(pool: &PgPool, id: i32) -> Result<Option<Self>, Error> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM interaction WHERE id = $1", id)
                .fetch_optional(pool)
                .await?,
        )
    }

    /// Mark an interaction as completed by setting its end time
    ///
    /// # Arguments
    ///
    /// * `pool`: The database pool to use
    pub async fn complete(&mut self, pool: &PgPool) -> Result<(), Error> {
        let ended = Utc::now();
        sqlx::query!(
            "UPDATE interaction SET ended = $1 WHERE id = $2",
            ended,
            self.id
        )
        .execute(pool)
        .await?;
        self.ended = Some(ended);

        Ok(())
    }
}
