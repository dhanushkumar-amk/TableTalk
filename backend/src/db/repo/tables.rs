use crate::db::{error::DbError, models::DiningTable};
use sqlx::SqlitePool;

pub async fn list_active_tables(pool: &SqlitePool) -> Result<Vec<DiningTable>, DbError> {
    let tables = sqlx::query_as::<_, DiningTable>(
        r#"
        SELECT id, label, capacity_min, capacity_max, area, is_active
        FROM dining_tables
        WHERE is_active = 1
        ORDER BY capacity_min ASC, label ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(tables)
}

pub async fn get_table_by_id(pool: &SqlitePool, id: i64) -> Result<DiningTable, DbError> {
    sqlx::query_as::<_, DiningTable>(
        r#"
        SELECT id, label, capacity_min, capacity_max, area, is_active
        FROM dining_tables
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Table with id {} not found", id)))
}

pub async fn get_table_by_label(pool: &SqlitePool, label: &str) -> Result<DiningTable, DbError> {
    sqlx::query_as::<_, DiningTable>(
        r#"
        SELECT id, label, capacity_min, capacity_max, area, is_active
        FROM dining_tables
        WHERE label = ?
        "#,
    )
    .bind(label)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Table with label '{}' not found", label)))
}
