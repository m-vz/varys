use std::env;

use log::{debug, info, trace};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Database, Execute, PgPool};

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

    info!("Connected to database at {url}");

    migrate(&pool).await?;

    Ok(pool)
}

pub async fn migrate(pool: &PgPool) -> Result<(), Error> {
    debug!("Migrating database if necessary");

    sqlx::migrate!("./migrations").run(pool).await?;

    Ok(())
}

fn log_query<'q, DB>(query: &impl Execute<'q, DB>)
where
    DB: Database,
{
    trace!("Running SQL: {}", query.sql());
}
