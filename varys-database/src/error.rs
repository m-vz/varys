use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    DatabaseMigration(#[from] sqlx::migrate::MigrateError),
    #[error("Environment variable DATABASE_URL is missing")]
    MissingDatabaseUrl,
}
