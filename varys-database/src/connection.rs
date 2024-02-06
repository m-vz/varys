use sqlx::PgPool;

pub struct DatabaseConnection {
    pub(crate) pool: PgPool,
}
