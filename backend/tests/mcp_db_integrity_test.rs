use chrono::{DateTime, TimeZone, Utc};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::str::FromStr;
use std::sync::Arc;
use tabletalk_backend::db::{
    error::DbError,
    init_pool,
    models::{Area, BookingStatus, DepositStatus},
    repo::{bookings, customers, menu, restaurant, tables},
    seed_database,
};

/// Helper to set up an in-memory database with foreign keys and migrations
async fn setup_test_db() -> SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    pool
}

/// Helper to set up an in-memory database with shared cache for concurrent tests
async fn setup_shared_test_db() -> (SqlitePool, tempfile::NamedTempFile) {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let conn_str = format!("sqlite://{}", db_path);

    let pool = init_pool(&conn_str).await.unwrap();
    (pool, temp_file)
}

#[tokio::test]
async fn test_migrations_apply_and_are_idempotent() {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();

    // First migration run
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Initial migrations should apply cleanly");

    // Second migration run (idempotency check)
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Rerunning migrations should succeed without errors");

    // Verify critical tables exist
    let table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('restaurant', 'bookings', 'menu_items', 'dining_tables')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(table_count, 4, "Core tables must exist in schema");
}

#[tokio::test]
async fn test_seeding_idempotence_and_counts() {
    let pool = setup_test_db().await;
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 15, 12, 0, 0).unwrap();

    // First seed
    seed_database(&pool, Some(fixed_now))
        .await
        .expect("First seed must succeed");

    let tables = [
        ("restaurant", 1),
        ("opening_hours", 14),
        ("special_closures", 1),
        ("dining_tables", 14),
        ("menu_categories", 6),
        ("dietary_tags", 6),
        ("allergens", 10),
        ("menu_items", 35),
        ("customers", 5),
        ("customer_preferences", 13),
        ("bookings", 5),
        ("orders", 2),
        ("order_items", 6),
        ("payments", 3),
    ];

    for (table, expected_count) in &tables {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count, *expected_count,
            "Table {} count mismatch after 1st seed",
            table
        );
    }

    // Second seed (idempotency test)
    seed_database(&pool, Some(fixed_now))
        .await
        .expect("Second seed must succeed");

    for (table, expected_count) in &tables {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count, *expected_count,
            "Table {} count changed after 2nd seed! Seeding is not idempotent",
            table
        );
    }
}

#[tokio::test]
async fn test_foreign_key_and_check_violations() {
    let pool = setup_test_db().await;
    seed_database(&pool, None).await.unwrap();

    // 1. Negative menu price violation
    let res = sqlx::query(
        r#"
        INSERT INTO menu_items (category_id, name, description, price_cents, currency, prep_time_minutes, spice_level)
        VALUES (1, 'Negative Dish', 'Invalid', -500, 'INR', 15, 1)
        "#,
    )
    .execute(&pool)
    .await;
    assert!(
        res.is_err(),
        "CHECK constraint must reject negative price_cents"
    );

    // 2. Booking party size zero violation
    let res = sqlx::query(
        r#"
        INSERT INTO bookings (id, confirmation_code, guest_name, party_size, start_at, end_at, status, created_at, updated_at)
        VALUES ('b-err-01', 'TT-ERR1', 'Invalid Party', 0, '2026-10-20T19:00:00Z', '2026-10-20T20:30:00Z', 'confirmed', '2026-10-15T00:00:00Z', '2026-10-15T00:00:00Z')
        "#,
    )
    .execute(&pool)
    .await;
    assert!(res.is_err(), "CHECK constraint must reject party_size <= 0");

    // 3. Booking end before start violation
    let res = sqlx::query(
        r#"
        INSERT INTO bookings (id, confirmation_code, guest_name, party_size, start_at, end_at, status, created_at, updated_at)
        VALUES ('b-err-02', 'TT-ERR2', 'Backwards Time', 2, '2026-10-20T21:00:00Z', '2026-10-20T20:00:00Z', 'confirmed', '2026-10-15T00:00:00Z', '2026-10-15T00:00:00Z')
        "#,
    )
    .execute(&pool)
    .await;
    assert!(
        res.is_err(),
        "CHECK constraint must reject end_at <= start_at"
    );

    // 4. Invalid booking status value
    let res = sqlx::query(
        r#"
        INSERT INTO bookings (id, confirmation_code, guest_name, party_size, start_at, end_at, status, created_at, updated_at)
        VALUES ('b-err-03', 'TT-ERR3', 'Invalid Status', 2, '2026-10-20T19:00:00Z', '2026-10-20T20:30:00Z', 'nonexistent_status', '2026-10-15T00:00:00Z', '2026-10-15T00:00:00Z')
        "#,
    )
    .execute(&pool)
    .await;
    assert!(res.is_err(), "CHECK constraint must reject unlisted status");

    // 5. Table capacity_min > capacity_max violation
    let res = sqlx::query(
        r#"
        INSERT INTO dining_tables (label, capacity_min, capacity_max, area)
        VALUES ('T-INV', 6, 2, 'indoor')
        "#,
    )
    .execute(&pool)
    .await;
    assert!(
        res.is_err(),
        "CHECK constraint must reject capacity_min > capacity_max"
    );

    // 6. Foreign key orphan reference
    let res = sqlx::query(
        r#"
        INSERT INTO menu_items (category_id, name, description, price_cents, currency, prep_time_minutes, spice_level)
        VALUES (999999, 'Orphan Item', 'No category', 25000, 'INR', 10, 0)
        "#,
    )
    .execute(&pool)
    .await;
    assert!(
        res.is_err(),
        "Foreign key constraint must reject non-existent category_id"
    );
}

#[tokio::test]
async fn test_allergen_and_dietary_tag_integrity() {
    let pool = setup_test_db().await;
    seed_database(&pool, None).await.unwrap();

    // 1. No item tagged 'vegan' can contain animal allergens ('dairy', 'egg', 'fish', 'shellfish')
    let invalid_vegan_items: Vec<(i64, String, String)> = sqlx::query_as(
        r#"
        SELECT mi.id, mi.name, a.name
        FROM menu_items mi
        INNER JOIN menu_item_dietary_tags midt ON midt.menu_item_id = mi.id
        INNER JOIN dietary_tags dt ON dt.id = midt.tag_id
        INNER JOIN menu_item_allergens mia ON mia.menu_item_id = mi.id
        INNER JOIN allergens a ON a.id = mia.allergen_id
        WHERE dt.name = 'vegan'
          AND a.name IN ('dairy', 'egg', 'fish', 'shellfish')
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert!(
        invalid_vegan_items.is_empty(),
        "CRITICAL ALLERGEN SAFETY FAILURE: Vegan items contain animal allergens: {:?}",
        invalid_vegan_items
    );

    // 2. No item tagged 'gluten_free' can contain allergen 'gluten'
    let invalid_gf_items: Vec<(i64, String, String)> = sqlx::query_as(
        r#"
        SELECT mi.id, mi.name, a.name
        FROM menu_items mi
        INNER JOIN menu_item_dietary_tags midt ON midt.menu_item_id = mi.id
        INNER JOIN dietary_tags dt ON dt.id = midt.tag_id
        INNER JOIN menu_item_allergens mia ON mia.menu_item_id = mi.id
        INNER JOIN allergens a ON a.id = mia.allergen_id
        WHERE dt.name = 'gluten_free'
          AND a.name = 'gluten'
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert!(
        invalid_gf_items.is_empty(),
        "CRITICAL ALLERGEN SAFETY FAILURE: Gluten-free items contain gluten allergen: {:?}",
        invalid_gf_items
    );

    // 3. No item tagged 'dairy_free' can contain allergen 'dairy'
    let invalid_df_items: Vec<(i64, String, String)> = sqlx::query_as(
        r#"
        SELECT mi.id, mi.name, a.name
        FROM menu_items mi
        INNER JOIN menu_item_dietary_tags midt ON midt.menu_item_id = mi.id
        INNER JOIN dietary_tags dt ON dt.id = midt.tag_id
        INNER JOIN menu_item_allergens mia ON mia.menu_item_id = mi.id
        INNER JOIN allergens a ON a.id = mia.allergen_id
        WHERE dt.name = 'dairy_free'
          AND a.name = 'dairy'
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert!(
        invalid_df_items.is_empty(),
        "CRITICAL ALLERGEN SAFETY FAILURE: Dairy-free items contain dairy allergen: {:?}",
        invalid_df_items
    );

    // 4. No item tagged 'nut_free' can contain allergens 'peanuts' or 'tree_nuts'
    let invalid_nf_items: Vec<(i64, String, String)> = sqlx::query_as(
        r#"
        SELECT mi.id, mi.name, a.name
        FROM menu_items mi
        INNER JOIN menu_item_dietary_tags midt ON midt.menu_item_id = mi.id
        INNER JOIN dietary_tags dt ON dt.id = midt.tag_id
        INNER JOIN menu_item_allergens mia ON mia.menu_item_id = mi.id
        INNER JOIN allergens a ON a.id = mia.allergen_id
        WHERE dt.name = 'nut_free'
          AND a.name IN ('peanuts', 'tree_nuts')
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert!(
        invalid_nf_items.is_empty(),
        "CRITICAL ALLERGEN SAFETY FAILURE: Nut-free items contain nut allergens: {:?}",
        invalid_nf_items
    );
}

#[tokio::test]
async fn test_concurrent_overlap_prevention_primitive() {
    let (pool, _temp_file) = setup_shared_test_db().await;
    seed_database(&pool, None).await.unwrap();

    let pool = Arc::new(pool);
    let target_table_id = 1; // T01
    let start_at = "2026-11-20T19:00:00Z";
    let end_at = "2026-11-20T20:30:00Z";
    let now_utc = "2026-10-15T12:00:00Z";

    let pool_1 = Arc::clone(&pool);
    let pool_2 = Arc::clone(&pool);

    let task1 = tokio::spawn(async move {
        bookings::try_reserve_table(
            &pool_1,
            "b-concurrent-001",
            "TT-RACE1",
            target_table_id,
            start_at,
            end_at,
            2,
            "Competitor One",
            None,
            Some("Window seat"),
            DepositStatus::None,
            0,
            now_utc,
        )
        .await
    });

    let task2 = tokio::spawn(async move {
        bookings::try_reserve_table(
            &pool_2,
            "b-concurrent-002",
            "TT-RACE2",
            target_table_id,
            start_at,
            end_at,
            2,
            "Competitor Two",
            None,
            Some("Any seat"),
            DepositStatus::None,
            0,
            now_utc,
        )
        .await
    });

    let (res1, res2) = tokio::join!(task1, task2);
    let r1 = res1.unwrap();
    let r2 = res2.unwrap();

    let (success_count, conflict_count) = match (&r1, &r2) {
        (Ok(_), Err(DbError::TableConflict { .. })) => (1, 1),
        (Err(DbError::TableConflict { .. }), Ok(_)) => (1, 1),
        _ => panic!(
            "Expected exactly one Ok and one TableConflict, got r1: {:?}, r2: {:?}",
            r1, r2
        ),
    };

    assert_eq!(success_count, 1);
    assert_eq!(conflict_count, 1);

    // Verify exactly 1 booking exists for that table and time in the database
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM bookings
        WHERE table_id = ? AND start_at = ?
        "#,
    )
    .bind(target_table_id)
    .bind(start_at)
    .fetch_one(&*pool)
    .await
    .unwrap();

    assert_eq!(count, 1, "Exactly one booking must persist in the database");
}

#[tokio::test]
async fn test_timezone_handling_and_utc_conversion() {
    use chrono_tz::Asia::Kolkata;

    // Normal afternoon service: 12:30 IST is 07:00 UTC
    let local_lunch = Kolkata.with_ymd_and_hms(2026, 10, 20, 12, 30, 0).unwrap();
    let utc_lunch: DateTime<Utc> = local_lunch.with_timezone(&Utc);
    assert_eq!(utc_lunch.to_rfc3339(), "2026-10-20T07:00:00+00:00");

    // Evening service: 19:30 IST is 14:00 UTC
    let local_dinner = Kolkata.with_ymd_and_hms(2026, 10, 20, 19, 30, 0).unwrap();
    let utc_dinner: DateTime<Utc> = local_dinner.with_timezone(&Utc);
    assert_eq!(utc_dinner.to_rfc3339(), "2026-10-20T14:00:00+00:00");

    // Late dinner / midnight boundary crossing:
    // A post-midnight slot at 00:30 IST on Oct 21 is 19:00 UTC on Oct 20!
    let local_late_night = Kolkata.with_ymd_and_hms(2026, 10, 21, 0, 30, 0).unwrap();
    let utc_late_night: DateTime<Utc> = local_late_night.with_timezone(&Utc);
    assert_eq!(utc_late_night.to_rfc3339(), "2026-10-20T19:00:00+00:00");
    assert_eq!(utc_late_night.date_naive().to_string(), "2026-10-20");
    assert_eq!(local_late_night.date_naive().to_string(), "2026-10-21");
}

#[tokio::test]
async fn test_repository_reads_seeded_data() {
    let pool = setup_test_db().await;
    seed_database(&pool, None).await.unwrap();

    // 1. Restaurant config
    let config = restaurant::get_config(&pool).await.unwrap();
    assert_eq!(config.name, "The Saffron Nebula Bistro");
    assert_eq!(config.timezone, "Asia/Kolkata");
    assert_eq!(config.currency, "INR");
    assert_eq!(config.dining_duration_minutes, 90);

    // 2. Opening hours & closures
    let hours = restaurant::get_opening_hours(&pool).await.unwrap();
    assert_eq!(hours.len(), 14);
    let closures = restaurant::get_special_closures(&pool).await.unwrap();
    assert_eq!(closures.len(), 1);

    // 3. Tables
    let active_tables = tables::list_active_tables(&pool).await.unwrap();
    assert_eq!(active_tables.len(), 14);
    let t01 = tables::get_table_by_label(&pool, "T01").await.unwrap();
    assert_eq!(t01.area, Area::Indoor);
    assert_eq!(t01.capacity_min, 2);

    // 4. Menu categories & items
    let categories = menu::list_categories(&pool).await.unwrap();
    assert_eq!(categories.len(), 6);

    let all_items = menu::list_menu_items(&pool, false).await.unwrap();
    assert_eq!(all_items.len(), 35);

    let available_items = menu::list_menu_items(&pool, true).await.unwrap();
    assert_eq!(available_items.len(), 33); // 2 unavailable items

    // 5. Customers & preferences
    let custs = customers::list_customers(&pool).await.unwrap();
    assert_eq!(custs.len(), 5);

    let rohan = customers::get_customer_by_id(&pool, 1).await.unwrap();
    assert_eq!(rohan.display_name, "Rohan Sharma");

    let prefs = customers::get_customer_preferences(&pool, 1).await.unwrap();
    assert_eq!(prefs.len(), 4);

    // 6. Bookings
    let booking = bookings::get_booking_by_code(&pool, "TT-8101")
        .await
        .unwrap();
    assert_eq!(booking.guest_name, "Rohan Sharma");
    assert_eq!(booking.status, BookingStatus::Completed);
}
