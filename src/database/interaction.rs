use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::error::Error;

#[derive(FromRow, Debug)]
pub struct Interaction {
    pub id: i32,
    pub started: DateTime<Utc>,
    #[sqlx(default)]
    pub ended: Option<DateTime<Utc>>,
}

impl Interaction {
    pub async fn create(
        started: DateTime<Utc>,
        ended: DateTime<Utc>,
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

        Ok(Interaction {
            id,
            started,
            ended: None,
        })
    }

    pub async fn get(id: i32, pool: &PgPool) -> Result<Option<Self>, Error> {
        Ok(
            sqlx::query_as!(Self, "SELECT * FROM interaction WHERE id = $1", id)
                .fetch_optional(pool)
                .await?,
        )
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

    pub fn completed(&self) -> bool {
        self.ended.is_some()
    }
}
