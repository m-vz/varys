use std::env;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::error::Error;

pub mod interaction;
pub mod interactor_config;
pub mod query;
pub mod session;

/// Connect to the database as specified in the environment variable `DATABASE_URL`.
///
/// This also migrates the database if there are any outstanding migrations.
pub async fn connect() -> Result<PgPool, Error> {
    let url = env::var("DATABASE_URL").map_err(|_| Error::MissingDatabaseUrl)?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url.as_str())
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
