use chrono::{DateTime, Local};
use sqlx::PgPool;

use crate::error::Error;

#[derive(Debug)]
pub struct Interaction {
    pub id: i32,
    pub started: DateTime<Local>,
    pub ended: DateTime<Local>,
}

impl Interaction {
    pub async fn create(
        started: DateTime<Local>,
        ended: DateTime<Local>,
        pool: &PgPool,
    ) -> Result<Self, Error> {
        let id = sqlx::query!(
            "INSERT INTO interaction (started, ended) VALUES ($1, $2) RETURNING id",
            started,
            ended,
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(Interaction { id, started, ended })
    }

    pub async fn update(&self, pool: &PgPool) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE interaction SET started = $1, ended = $2 WHERE id = $3",
            self.started,
            self.ended,
            self.id,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get(id: i32, pool: &PgPool) -> Result<Option<Self>, Error> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM interaction WHERE id = $1", id)
                .fetch_optional(pool)
                .await?,
        )
    }
}
