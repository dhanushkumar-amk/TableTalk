use crate::db::{
    error::DbError,
    models::{Order, OrderItem, Payment},
};
use sqlx::SqlitePool;

pub async fn list_orders(pool: &SqlitePool) -> Result<Vec<Order>, DbError> {
    let orders = sqlx::query_as::<_, Order>(
        r#"
        SELECT id, order_code, customer_id, booking_id, order_type, status, pickup_time,
               subtotal_cents, tax_cents, tip_cents, total_cents, currency,
               notes, guest_name, guest_phone, guest_email, stated_allergens,
               created_at, updated_at, cancelled_at
        FROM orders
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(orders)
}

pub async fn get_order_by_id(pool: &SqlitePool, id: &str) -> Result<Order, DbError> {
    sqlx::query_as::<_, Order>(
        r#"
        SELECT id, order_code, customer_id, booking_id, order_type, status, pickup_time,
               subtotal_cents, tax_cents, tip_cents, total_cents, currency,
               notes, guest_name, guest_phone, guest_email, stated_allergens,
               created_at, updated_at, cancelled_at
        FROM orders
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Order with id '{}' not found", id)))
}

pub async fn get_order_by_code(pool: &SqlitePool, order_code: &str) -> Result<Order, DbError> {
    sqlx::query_as::<_, Order>(
        r#"
        SELECT id, order_code, customer_id, booking_id, order_type, status, pickup_time,
               subtotal_cents, tax_cents, tip_cents, total_cents, currency,
               notes, guest_name, guest_phone, guest_email, stated_allergens,
               created_at, updated_at, cancelled_at
        FROM orders
        WHERE order_code = ?
        "#,
    )
    .bind(order_code)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Order with code '{}' not found", order_code)))
}

pub async fn get_order_items(pool: &SqlitePool, order_id: &str) -> Result<Vec<OrderItem>, DbError> {
    let items = sqlx::query_as::<_, OrderItem>(
        r#"
        SELECT id, order_id, menu_item_id, item_name, quantity, unit_price_cents, currency, notes
        FROM order_items
        WHERE order_id = ?
        ORDER BY id ASC
        "#,
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;

    Ok(items)
}

pub async fn list_payments_for_order(
    pool: &SqlitePool,
    order_id: &str,
) -> Result<Vec<Payment>, DbError> {
    let payments = sqlx::query_as::<_, Payment>(
        r#"
        SELECT id, booking_id, order_id, session_id, kind, amount_minor, currency,
               provider, provider_reference, status, payer_name, payer_contact,
               created_at, updated_at
        FROM payments
        WHERE order_id = ?
        ORDER BY created_at ASC
        "#,
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;

    Ok(payments)
}
