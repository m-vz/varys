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
    pub session_id: i32,
    pub query: String,
    pub response: Option<String>,
    pub started: DateTime<Utc>,
    #[sqlx(default)]
    pub ended: Option<DateTime<Utc>>,
}

impl Interaction {
    /// Create a new interaction in the database.
    ///
    /// # Arguments
    ///
    /// * `pool`: The database pool to use.
    /// * `session`: The session to associate the interaction with.
    pub async fn create(pool: &PgPool, session: &Session, query: &str) -> Result<Self, Error> {
        let started = Utc::now();
        let id = sqlx::query!(
            "INSERT INTO interaction (started, session_id, query) VALUES ($1, $2, $3) RETURNING id",
            started,
            session.id,
            query,
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(Interaction {
            id,
            session_id: session.id,
            query: query.to_string(),
            response: None,
            started,
            ended: None,
        })
    }

    /// Get an interaction from the database.
    ///
    /// # Arguments
    ///
    /// * `pool`: The database pool to use.
    /// * `id`: The id of the interaction.
    pub async fn get(pool: &PgPool, id: i32) -> Result<Option<Self>, Error> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM interaction WHERE id = $1", id)
                .fetch_optional(pool)
                .await?,
        )
    }

    /// Mark an interaction as completed by setting its end time.
    ///
    /// # Arguments
    ///
    /// * `pool`: The database pool to use.
    pub async fn complete(&mut self, pool: &PgPool) -> Result<&mut Self, Error> {
        let ended = Utc::now();
        sqlx::query!(
            "UPDATE interaction SET ended = $1 WHERE id = $2",
            ended,
            self.id,
        )
        .execute(pool)
        .await?;
        self.ended = Some(ended);

        Ok(self)
    }

    pub async fn add_response(
        &mut self,
        pool: &PgPool,
        response: &str,
    ) -> Result<&mut Self, Error> {
        sqlx::query!(
            "UPDATE interaction SET response = $1 WHERE id = $2",
            response,
            self.id
        )
        .execute(pool)
        .await?;
        self.response = Some(response.to_string());

        Ok(self)
    }
}
