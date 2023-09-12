use std::env;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::error::Error;

pub mod interaction;
pub mod session;

pub async fn connect() -> Result<PgPool, Error> {
    let url = env::var("DATABASE_URL").map_err(|_| Error::MissingDatabaseUrl)?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url.as_str())
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
