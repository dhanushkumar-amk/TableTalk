use crate::db::{
    error::DbError,
    models::{Customer, CustomerPreference},
};
use sqlx::SqlitePool;

pub async fn list_customers(pool: &SqlitePool) -> Result<Vec<Customer>, DbError> {
    let customers = sqlx::query_as::<_, Customer>(
        r#"
        SELECT id, display_name, phone, email, created_at
        FROM customers
        ORDER BY id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(customers)
}

pub async fn get_customer_by_id(pool: &SqlitePool, id: i64) -> Result<Customer, DbError> {
    sqlx::query_as::<_, Customer>(
        r#"
        SELECT id, display_name, phone, email, created_at
        FROM customers
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Customer with id {} not found", id)))
}

pub async fn get_customer_preferences(
    pool: &SqlitePool,
    customer_id: i64,
) -> Result<Vec<CustomerPreference>, DbError> {
    let prefs = sqlx::query_as::<_, CustomerPreference>(
        r#"
        SELECT id, customer_id, kind, value, source, created_at
        FROM customer_preferences
        WHERE customer_id = ?
        ORDER BY id ASC
        "#,
    )
    .bind(customer_id)
    .fetch_all(pool)
    .await?;

    Ok(prefs)
}
