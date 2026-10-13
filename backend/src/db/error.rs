use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error(
        "Table conflict: table {table_id} is already reserved for period {start_at} to {end_at}"
    )]
    TableConflict {
        table_id: i64,
        start_at: String,
        end_at: String,
    },

    #[error("Validation error: {0}")]
    Validation(String),
}
