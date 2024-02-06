use std::env;

use log::{debug, info, trace};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Database, Execute};

use crate::connection::DatabaseConnection;
use crate::error::DatabaseError;

pub mod interaction;
pub mod interactor_config;
pub mod session;

/// Connect to the database as specified in the environment variable `DATABASE_URL`.
///
/// This also migrates the database if there are any outstanding migrations.
pub async fn connect() -> Result<DatabaseConnection, DatabaseError> {
    let url = env::var("DATABASE_URL").map_err(|_| DatabaseError::MissingDatabaseUrl)?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url.as_str())
        .await?;
    let connection = DatabaseConnection { pool };

    info!("Connected to database at {url}");

    migrate(&connection).await?;

    Ok(connection)
}

pub async fn migrate(connection: &DatabaseConnection) -> Result<(), DatabaseError> {
    debug!("Migrating database if necessary...");

    sqlx::migrate!("./migrations").run(&connection.pool).await?;

    Ok(())
}

fn log_query<'q, DB>(query: &impl Execute<'q, DB>)
where
    DB: Database,
{
    trace!("Running SQL: {}", query.sql());
}
