use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::database::interaction::Interaction;
use crate::error::Error;

#[derive(FromRow, Debug)]
pub struct Session {
    pub id: i32,
    pub started: DateTime<Utc>,
    #[sqlx(default)]
    pub ended: Option<DateTime<Utc>>,
}

impl Session {
    pub async fn new(pool: &PgPool) -> Result<Self, Error> {
        let started = Utc::now();
        let id = sqlx::query!(
            "INSERT INTO session (started) VALUES ($1) RETURNING id",
            started,
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(Session {
            id,
            started,
            ended: None,
        })
    }

    pub async fn get(id: i32, pool: &PgPool) -> Result<Option<Self>, Error> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM session WHERE id = $1", id)
                .fetch_optional(pool)
                .await?,
        )
    }

    pub async fn new_interaction(&self, pool: &PgPool) -> Result<Interaction, Error> {
        let mut interaction = Interaction::create(pool, self).await?;

        interaction.add_session(pool, self).await?;

        Ok(interaction)
    }

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
