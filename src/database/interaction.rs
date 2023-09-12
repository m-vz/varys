use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::database::session::Session;
use crate::error::Error;

#[derive(FromRow, Debug)]
pub struct Interaction {
    pub id: i32,
    pub started: DateTime<Utc>,
    #[sqlx(default)]
    pub ended: Option<DateTime<Utc>>,
    pub session_id: i32,
}

impl Interaction {
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

    pub async fn get(pool: &PgPool, id: i32) -> Result<Option<Self>, Error> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM interaction WHERE id = $1", id)
                .fetch_optional(pool)
                .await?,
        )
    }

    pub async fn add_session(&mut self, pool: &PgPool, session: &Session) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE interaction SET session_id = $1 WHERE id = $2",
            session.id,
            self.id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

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
