use crate::db::{
    error::DbError,
    models::{OpeningHours, RestaurantConfig, SpecialClosure},
};
use sqlx::SqlitePool;

pub async fn get_config(pool: &SqlitePool) -> Result<RestaurantConfig, DbError> {
    sqlx::query_as::<_, RestaurantConfig>(
        r#"
        SELECT id, name, timezone, currency, dining_duration_minutes, slot_granularity_minutes,
               max_party_size, lead_time_minutes, booking_horizon_days, deposit_required_party_size,
               deposit_amount_per_person_cents, created_at, updated_at,
               cancellation_cutoff_hours, late_cancellation_fee_cents
        FROM restaurant
        WHERE id = 1
        "#,
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound("Restaurant configuration not initialized".to_string()))
}

pub async fn get_opening_hours(pool: &SqlitePool) -> Result<Vec<OpeningHours>, DbError> {
    let hours = sqlx::query_as::<_, OpeningHours>(
        r#"
        SELECT id, day_of_week, period_name, open_time, close_time, is_closed
        FROM opening_hours
        ORDER BY day_of_week ASC, open_time ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(hours)
}

pub async fn get_special_closures(pool: &SqlitePool) -> Result<Vec<SpecialClosure>, DbError> {
    let closures = sqlx::query_as::<_, SpecialClosure>(
        r#"
        SELECT id, closure_date, reason, is_all_day, open_time, close_time
        FROM special_closures
        ORDER BY closure_date ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(closures)
}
