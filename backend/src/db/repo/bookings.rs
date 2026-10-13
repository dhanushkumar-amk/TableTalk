use crate::db::{
    error::DbError,
    models::{Booking, DepositStatus, BOOKING_SELECT_COLUMNS},
};
use sqlx::SqlitePool;

pub async fn list_bookings(pool: &SqlitePool) -> Result<Vec<Booking>, DbError> {
    let bookings = sqlx::query_as::<_, Booking>(&format!(
        "SELECT {} FROM bookings ORDER BY start_at ASC",
        BOOKING_SELECT_COLUMNS
    ))
    .fetch_all(pool)
    .await?;

    Ok(bookings)
}

pub async fn get_booking_by_id(pool: &SqlitePool, id: &str) -> Result<Booking, DbError> {
    sqlx::query_as::<_, Booking>(&format!(
        "SELECT {} FROM bookings WHERE id = ?",
        BOOKING_SELECT_COLUMNS
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Booking with id '{}' not found", id)))
}

pub async fn get_booking_by_code(pool: &SqlitePool, code: &str) -> Result<Booking, DbError> {
    sqlx::query_as::<_, Booking>(&format!(
        "SELECT {} FROM bookings WHERE confirmation_code = ?",
        BOOKING_SELECT_COLUMNS
    ))
    .bind(code)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Booking with code '{}' not found", code)))
}

pub async fn list_bookings_by_customer(
    pool: &SqlitePool,
    customer_id: i64,
) -> Result<Vec<Booking>, DbError> {
    let bookings = sqlx::query_as::<_, Booking>(&format!(
        "SELECT {} FROM bookings WHERE customer_id = ? ORDER BY start_at ASC",
        BOOKING_SELECT_COLUMNS
    ))
    .bind(customer_id)
    .fetch_all(pool)
    .await?;

    Ok(bookings)
}

/// List active (pending or confirmed) bookings that overlap the specified time window [start_iso, end_iso).
/// Overlap condition: start_at < end_iso AND end_at > start_iso.
///
/// Lazy deposit expiry: a booking that is still pending and awaiting a deposit whose
/// payment deadline has passed no longer blocks the slot, even before the sweeper runs.
pub async fn list_active_bookings_in_range(
    pool: &SqlitePool,
    start_iso: &str,
    end_iso: &str,
) -> Result<Vec<Booking>, DbError> {
    let bookings = sqlx::query_as::<_, Booking>(&format!(
        r#"
        SELECT {cols} FROM bookings
        WHERE status IN ('pending', 'confirmed')
          AND NOT (status = 'pending' AND deposit_status = 'requires_payment'
                   AND payment_deadline_at IS NOT NULL AND payment_deadline_at <= ?)
          AND start_at < ?
          AND end_at > ?
        ORDER BY start_at ASC
        "#,
        cols = BOOKING_SELECT_COLUMNS
    ))
    .bind(start_iso)
    .bind(end_iso)
    .bind(start_iso)
    .fetch_all(pool)
    .await?;

    Ok(bookings)
}

/// Double-booking prevention primitive using SQLite BEGIN IMMEDIATE transaction semantics.
/// Serializes concurrent booking attempts for the same table across active bookings.
#[allow(clippy::too_many_arguments)]
pub async fn try_reserve_table(
    pool: &SqlitePool,
    id: &str,
    confirmation_code: &str,
    table_id: i64,
    start_at: &str,
    end_at: &str,
    party_size: i64,
    guest_name: &str,
    customer_id: Option<i64>,
    special_requests: Option<&str>,
    deposit_status: DepositStatus,
    deposit_amount_minor: i64,
    now_utc: &str,
) -> Result<Booking, DbError> {
    let mut conn = pool.acquire().await?;

    // Acquire exclusive reservation lock via SQLite BEGIN IMMEDIATE
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    let res: Result<Booking, DbError> = async {
        // Check for any active overlapping booking on this table
        // Overlap definition: (new_start < existing_end) AND (new_end > existing_start)
        let overlap_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM bookings
            WHERE table_id = ?
              AND status IN ('pending', 'confirmed')
              AND start_at < ?
              AND end_at > ?
            "#,
        )
        .bind(table_id)
        .bind(end_at)
        .bind(start_at)
        .fetch_one(&mut *conn)
        .await?;

        if overlap_count > 0 {
            return Err(DbError::TableConflict {
                table_id,
                start_at: start_at.to_string(),
                end_at: end_at.to_string(),
            });
        }

        sqlx::query(
            r#"
            INSERT INTO bookings (
                id, confirmation_code, customer_id, guest_name, party_size,
                start_at, end_at, status, table_id, special_requests,
                deposit_status, deposit_amount_minor, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, 'confirmed', ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(confirmation_code)
        .bind(customer_id)
        .bind(guest_name)
        .bind(party_size)
        .bind(start_at)
        .bind(end_at)
        .bind(table_id)
        .bind(special_requests)
        .bind(deposit_status)
        .bind(deposit_amount_minor)
        .bind(now_utc)
        .bind(now_utc)
        .execute(&mut *conn)
        .await?;

        let booking = sqlx::query_as::<_, Booking>(&format!(
            "SELECT {} FROM bookings WHERE id = ?",
            BOOKING_SELECT_COLUMNS
        ))
        .bind(id)
        .fetch_one(&mut *conn)
        .await?;

        Ok(booking)
    }
    .await;

    match res {
        Ok(booking) => {
            sqlx::query("COMMIT").execute(&mut *conn).await?;
            Ok(booking)
        }
        Err(e) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(e)
        }
    }
}
