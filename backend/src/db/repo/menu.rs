use crate::db::{
    error::DbError,
    models::{Allergen, DietaryTag, MenuCategory, MenuItem, MenuItemWithDetails},
};
use sqlx::SqlitePool;

pub async fn list_categories(pool: &SqlitePool) -> Result<Vec<MenuCategory>, DbError> {
    let categories = sqlx::query_as::<_, MenuCategory>(
        r#"
        SELECT id, name, description, display_order
        FROM menu_categories
        ORDER BY display_order ASC, name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(categories)
}

pub async fn list_dietary_tags(pool: &SqlitePool) -> Result<Vec<DietaryTag>, DbError> {
    let tags = sqlx::query_as::<_, DietaryTag>(
        r#"
        SELECT id, name, description
        FROM dietary_tags
        ORDER BY name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(tags)
}

pub async fn list_allergens(pool: &SqlitePool) -> Result<Vec<Allergen>, DbError> {
    let allergens = sqlx::query_as::<_, Allergen>(
        r#"
        SELECT id, name, description
        FROM allergens
        ORDER BY name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(allergens)
}

pub async fn list_menu_items(
    pool: &SqlitePool,
    only_available: bool,
) -> Result<Vec<MenuItemWithDetails>, DbError> {
    let items = if only_available {
        sqlx::query_as::<_, MenuItem>(
            r#"
            SELECT id, category_id, name, description, price_cents, currency,
                   is_available, prep_time_minutes, image_url, spice_level, calories, is_chefs_special, allergens_verified
            FROM menu_items
            WHERE is_available = 1
            ORDER BY category_id ASC, id ASC
            "#,
        )
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, MenuItem>(
            r#"
            SELECT id, category_id, name, description, price_cents, currency,
                   is_available, prep_time_minutes, image_url, spice_level, calories, is_chefs_special, allergens_verified
            FROM menu_items
            ORDER BY category_id ASC, id ASC
            "#,
        )
        .fetch_all(pool)
        .await?
    };

    let mut result = Vec::with_capacity(items.len());
    for item in items {
        let details = get_menu_item_details(pool, item).await?;
        result.push(details);
    }

    Ok(result)
}

pub async fn get_menu_item_by_id(
    pool: &SqlitePool,
    id: i64,
) -> Result<MenuItemWithDetails, DbError> {
    let item = sqlx::query_as::<_, MenuItem>(
        r#"
        SELECT id, category_id, name, description, price_cents, currency,
               is_available, prep_time_minutes, image_url, spice_level, calories, is_chefs_special, allergens_verified
        FROM menu_items
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Menu item with id {} not found", id)))?;

    get_menu_item_details(pool, item).await
}

async fn get_menu_item_details(
    pool: &SqlitePool,
    item: MenuItem,
) -> Result<MenuItemWithDetails, DbError> {
    let category_name: String = sqlx::query_scalar(
        r#"
        SELECT name FROM menu_categories WHERE id = ?
        "#,
    )
    .bind(item.category_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_else(|| "Unknown".to_string());

    let dietary_tags: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT dt.name
        FROM dietary_tags dt
        INNER JOIN menu_item_dietary_tags midt ON midt.tag_id = dt.id
        WHERE midt.menu_item_id = ?
        ORDER BY dt.name ASC
        "#,
    )
    .bind(item.id)
    .fetch_all(pool)
    .await?;

    let allergens: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT a.name
        FROM allergens a
        INNER JOIN menu_item_allergens mia ON mia.allergen_id = a.id
        WHERE mia.menu_item_id = ?
        ORDER BY a.name ASC
        "#,
    )
    .bind(item.id)
    .fetch_all(pool)
    .await?;

    Ok(MenuItemWithDetails {
        item,
        category_name,
        dietary_tags,
        allergens,
    })
}
