use crate::db::{
    error::DbError,
    models::{
        Actor, Area, BookingStatus, DepositStatus, OrderStatus, OrderType, PaymentStatus,
        PreferenceKind, PreferenceSource,
    },
};
use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;

pub async fn seed_database(
    pool: &SqlitePool,
    now_override: Option<DateTime<Utc>>,
) -> Result<(), DbError> {
    let now = now_override.unwrap_or_else(Utc::now);
    let now_iso = now.to_rfc3339();

    // 1. Restaurant Config (Idempotent single row)
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO restaurant (
            id, name, timezone, currency, dining_duration_minutes, slot_granularity_minutes,
            max_party_size, lead_time_minutes, booking_horizon_days, deposit_required_party_size,
            deposit_amount_per_person_cents, created_at, updated_at
        )
        VALUES (1, 'The Saffron Nebula Bistro', 'Asia/Kolkata', 'INR', 90, 15, 12, 30, 30, 6, 50000, ?, ?)
        "#,
    )
    .bind(&now_iso)
    .bind(&now_iso)
    .execute(pool)
    .await?;

    // 2. Opening Hours (14 rows: Mon-Sun, 2 periods per day, Tuesdays closed)
    let hours_data = [
        (1, 0, "Lunch", "12:00", "15:30", 0),
        (2, 0, "Dinner", "19:00", "23:00", 0),
        (3, 1, "Lunch", "12:00", "15:30", 0),
        (4, 1, "Dinner", "19:00", "23:00", 0),
        (5, 2, "Lunch", "00:00", "00:00", 1), // Tuesday closed
        (6, 2, "Dinner", "00:00", "00:00", 1),
        (7, 3, "Lunch", "12:00", "15:30", 0),
        (8, 3, "Dinner", "19:00", "23:00", 0),
        (9, 4, "Lunch", "12:00", "15:30", 0),
        (10, 4, "Dinner", "19:00", "23:00", 0),
        (11, 5, "Lunch", "12:00", "15:30", 0),
        (12, 5, "Dinner", "19:00", "23:30", 0),
        (13, 6, "Lunch", "12:00", "16:00", 0),
        (14, 6, "Dinner", "19:00", "23:30", 0),
    ];

    for (id, dow, name, open, close, closed) in hours_data {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO opening_hours (id, day_of_week, period_name, open_time, close_time, is_closed)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(dow)
        .bind(name)
        .bind(open)
        .bind(close)
        .bind(closed)
        .execute(pool)
        .await?;
    }

    // 3. Special Closure (1 row, 14 days ahead)
    let closure_date = (now + Duration::days(14)).format("%Y-%m-%d").to_string();
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO special_closures (id, closure_date, reason, is_all_day, open_time, close_time)
        VALUES (1, ?, 'Annual Facility Maintenance & Culinary Development', 1, NULL, NULL)
        "#,
    )
    .bind(closure_date)
    .execute(pool)
    .await?;

    // 4. Dining Tables (14 tables: 6 indoor, 5 patio, 3 bar)
    let tables_data = [
        (1, "T01", 2, 2, Area::Indoor, 1),
        (2, "T02", 2, 2, Area::Indoor, 1),
        (3, "T03", 2, 4, Area::Indoor, 1),
        (4, "T04", 2, 4, Area::Indoor, 1),
        (5, "T05", 4, 6, Area::Indoor, 1),
        (6, "T06", 6, 8, Area::Indoor, 1),
        (7, "P01", 2, 2, Area::Patio, 1),
        (8, "P02", 2, 4, Area::Patio, 1),
        (9, "P03", 2, 4, Area::Patio, 1),
        (10, "P04", 4, 6, Area::Patio, 1),
        (11, "P05", 6, 8, Area::Patio, 1),
        (12, "B01", 1, 2, Area::Bar, 1),
        (13, "B02", 1, 2, Area::Bar, 1),
        (14, "B03", 2, 4, Area::Bar, 1),
    ];

    for (id, label, min_c, max_c, area, active) in tables_data {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO dining_tables (id, label, capacity_min, capacity_max, area, is_active)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(label)
        .bind(min_c)
        .bind(max_c)
        .bind(area)
        .bind(active)
        .execute(pool)
        .await?;
    }

    // 5. Menu Categories (6 categories)
    let categories_data = [
        (
            1,
            "Small Plates & Starters",
            "Crisp appetizers, chaats, and street food classics",
            1,
        ),
        (
            2,
            "Tandoor & Charcoal Grills",
            "Char-roasted kebabs, paneer tikka, and tandoori delicacies",
            2,
        ),
        (
            3,
            "Signature Curries & Mains",
            "Slow-simmered rich curries, dal makhani, and regional gravies",
            3,
        ),
        (
            4,
            "Biryani, Rice & Artisanal Breads",
            "Fragrant dum biryanis, sourdough naans, and rotis",
            4,
        ),
        (
            5,
            "Desserts & Sweet Finales",
            "Decadent Indian confections with modern artisanal touches",
            5,
        ),
        (
            6,
            "Beverages, Mocktails & Chai",
            "House-crafted coolers, spiced tonics, and single-estate teas",
            6,
        ),
    ];

    for (id, name, desc, order) in categories_data {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO menu_categories (id, name, description, display_order)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(desc)
        .bind(order)
        .execute(pool)
        .await?;
    }

    // 6. Dietary Tags (6 tags)
    let dietary_tags_data = [
        (
            1,
            "vegan",
            "100% plant-based, contains no animal flesh, dairy, eggs, or honey",
        ),
        (
            2,
            "vegetarian",
            "Contains no meat, poultry, or fish; may contain dairy",
        ),
        (
            3,
            "gluten_free",
            "Prepared without wheat, barley, rye, or other gluten-containing grains",
        ),
        (
            4,
            "dairy_free",
            "Free from mammalian milk, butter, ghee, cream, yogurt, or cheese",
        ),
        (
            5,
            "halal",
            "Prepared in strict accordance with halal culinary standards",
        ),
        (6, "nut_free", "Prepared without tree nuts or peanuts"),
    ];

    for (id, name, desc) in dietary_tags_data {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO dietary_tags (id, name, description)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(desc)
        .execute(pool)
        .await?;
    }

    // 7. Allergens (10 allergens)
    let allergens_data = [
        (
            1,
            "peanuts",
            "Peanuts and peanut-derived oils or ingredients",
        ),
        (2, "tree_nuts", "Cashews, almonds, pistachios, walnuts"),
        (3, "dairy", "Milk, cream, butter, ghee, paneer, yogurt"),
        (4, "gluten", "Wheat flour, maida, semolina"),
        (5, "soy", "Soybeans, edamame, tofu, soy sauce"),
        (6, "sesame", "Sesame seeds, sesame oil, tahini"),
        (7, "mustard", "Mustard seeds and cold-pressed mustard oil"),
        (8, "shellfish", "Prawns, crab, shrimp"),
        (9, "egg", "Eggs and egg derivatives"),
        (10, "fish", "Finfish, anchovies"),
    ];

    for (id, name, desc) in allergens_data {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO allergens (id, name, description)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(desc)
        .execute(pool)
        .await?;
    }

    // 8. Menu Items (35 items)
    // Structure: (id, category_id, name, description, price_cents, currency, is_available, prep_time, image_url, spice_level, calories, is_chefs_special)
    let items_data = [
        // Starters (Cat 1)
        (1, 1, "Crisp Palak Chaat", "Flash-fried spinach leaves drizzled with spiced yogurt, tamarind reduction, and pomegranate pearls", 38000, "INR", 1, 12, Some("https://images.example.com/palak-chaat.jpg"), 1, Some(310), 1),
        (2, 1, "Dahi Puri Spheres", "Semolina shells filled with spiced potato mash, sweet curd foam, and tangy mint water", 32000, "INR", 1, 10, None, 1, Some(260), 0),
        (3, 1, "Gunpowder Idli Bites", "Mini steamed rice cakes tossed in toasted podi masala and curry leaf coconut oil", 29000, "INR", 1, 10, None, 2, Some(220), 0),
        (4, 1, "Amritsari Fish Fingers", "Crispy ajwain-battered river sole with smoked radish pickle and lemon tartar", 46000, "INR", 1, 15, None, 2, Some(390), 0),
        (5, 1, "Kurkuri Bhindi Tartlets", "Spiced shredded okra straws served over micro-herb salad and mango chutney", 34000, "INR", 1, 12, None, 1, Some(190), 0),
        (6, 1, "Truffle Mushroom Galouti", "Silken melt-in-the-mouth smoked portobello patties on miniature saffron sheermal", 45000, "INR", 1, 18, None, 1, Some(340), 1),

        // Tandoor & Grills (Cat 2)
        (7, 2, "Paneer Tikka Angaare", "Charcoal-smoked cottage cheese cubes marinated in Kashmiri chili and hung curd", 48000, "INR", 1, 20, Some("https://images.example.com/paneer-tikka.jpg"), 2, Some(460), 0),
        (8, 2, "Tandoori Malai Broccoli", "Broccoli florets smothered with cardamom cream, cream cheese, and toasted almond flakes", 44000, "INR", 1, 18, None, 0, Some(380), 0),
        (9, 2, "Bhatti Ka Murgh", "Robust tandoori chicken leg quarters marinated in pounded spices and black cardamom", 56000, "INR", 1, 25, None, 3, Some(540), 0),
        (10, 2, "Murgh Malai Kebab", "Tender chicken suprêmes glazed with cashew cream, green cardamom, and garlic butter", 54000, "INR", 1, 22, None, 0, Some(510), 0),
        (11, 2, "Kakori Seekh Kebab", "Delicately minced goat meat skewers infused with saffron, rose petal, and cloves", 68000, "INR", 1, 25, None, 1, Some(580), 1),
        (12, 2, "Tiger Prawns Ajwaini", "Jumbo bay prawns blistered over open charcoal with carom seeds and yellow chili", 78000, "INR", 1, 20, None, 2, Some(420), 0),

        // Curries & Mains (Cat 3)
        (13, 3, "Nebula Dal Makhani", "Black lentils slow-simmered for 36 hours over clay charcoal with fresh churned butter", 42000, "INR", 1, 15, Some("https://images.example.com/dal-makhani.jpg"), 1, Some(480), 1),
        (14, 3, "Paneer Lababdar", "Handcrafted cottage cheese in a velvety tomato-onion gravy scented with dried fenugreek", 49000, "INR", 1, 18, None, 1, Some(520), 0),
        (15, 3, "Banarasi Baingan Bharta", "Smoked aubergine mash with green peas, charred shallots, and ginger juliennes", 39000, "INR", 1, 15, None, 2, Some(240), 0),
        (16, 3, "Pindi Chole Rawalpindi", "Dark slow-cooked chickpeas steeped in black tea broth, amchur, and caraway", 38000, "INR", 1, 15, None, 2, Some(350), 0),
        (17, 3, "Old Delhi Butter Chicken", "Pulled tandoori chicken simmered in silky strained plum tomato reduction and honey", 59000, "INR", 1, 20, Some("https://images.example.com/butter-chicken.jpg"), 1, Some(620), 0),
        (18, 3, "Kashmiri Rogan Josh", "Slow-braised spring lamb shanks in an aromatic liquor of Kashmiri mawal flower and fennel", 69000, "INR", 1, 25, None, 2, Some(660), 1),
        (19, 3, "Kerala Coconut Fish Moilee", "Seared kingfish fillet poached in light coconut cream, turmeric, and green chilies", 64000, "INR", 1, 22, None, 1, Some(490), 0),
        (20, 3, "Niligiri Veg Korma", "Seasonal harvest vegetables in a fragrant mint, cilantro, and roasted coconut velouté", 42000, "INR", 1, 18, None, 1, Some(310), 0),

        // Biryani, Rice & Breads (Cat 4)
        (21, 4, "Nizami Subz Dum Biryani", "Aged long-grain basmati layered with seasonal vegetables, saffron milk, and fried onions", 46000, "INR", 1, 25, None, 1, Some(540), 0),
        (22, 4, "Awadhi Dum Murgh Biryani", "Bone-in chicken and aged basmati rice cooked in sealed dough purdah with kewra water", 58000, "INR", 1, 25, Some("https://images.example.com/biryani.jpg"), 2, Some(670), 0),
        (23, 4, "Dum Gosht Biryani", "Tender mutton chunks steeped overnight in spice yogurt and layered with aromatic rice", 68000, "INR", 1, 30, None, 2, Some(750), 1),
        (24, 4, "Steamed Jeera Basmati", "Fluffy aged basmati rice tempered with shahi jeera and clarified butter", 22000, "INR", 1, 10, None, 0, Some(230), 0),
        (25, 4, "Truffle Garlic Butter Naan", "Tandoor-baked leavened flatbread brushed with black truffle butter and minced garlic", 18000, "INR", 1, 8, None, 0, Some(280), 0),
        (26, 4, "Laccha Paratha", "Flaky multi-layered whole wheat bread dusted with crushed dried mint", 14000, "INR", 1, 8, None, 0, Some(240), 0),
        (27, 4, "Tandoori Roti", "Whole wheat unleavened bread baked crisp on tandoor walls", 9000, "INR", 1, 6, None, 0, Some(140), 0),

        // Desserts (Cat 5)
        (28, 5, "Saffron Baked Gulab Jamun", "Mawa dumplings drenched in saffron honey syrup, baked in a rich rabri custard", 28000, "INR", 1, 10, None, 0, Some(410), 0),
        (29, 5, "Pistachio Kulfi Pop", "Traditional stone-churned dense frozen cream with roasted Iranian pistachios", 24000, "INR", 1, 5, None, 0, Some(320), 0),
        (30, 5, "Dark Chocolate Rasgulla Mousse", "Spongy cottage cheese spheres resting in 70% dark single-origin chocolate mousse", 32000, "INR", 1, 8, None, 0, Some(360), 1),
        (31, 5, "Vegan Coconut Mango Kheer", "Coconut milk rice pudding sweetened with organic palm jaggery and Alphonso mango", 26000, "INR", 1, 8, None, 0, Some(270), 0),

        // Beverages (Cat 6)
        (32, 6, "Kokum Cumin Cooler", "Wild kokum extract infused with roasted cumin, pink salt, and sparkling soda", 19000, "INR", 1, 5, None, 0, Some(90), 0),
        (33, 6, "Kesar Badam Thandai", "Chilled almond and melon seed elixir flavored with saffron, rose petals, and fennel", 24000, "INR", 1, 5, None, 0, Some(220), 0),
        (34, 6, "Darjeeling First Flush Tea", "Single-estate high-grown black tea served with wild forest honey and lemon", 18000, "INR", 1, 7, None, 0, Some(15), 0),
        (35, 6, "Masala Karak Chai", "Slow-brewed Assam CTC milk tea infused with green cardamom, fresh ginger, and cinnamon", 14000, "INR", 1, 8, None, 0, Some(110), 0),
    ];

    for (id, cat_id, name, desc, price, curr, avail, prep, img, spice, cal, special) in items_data {
        sqlx::query(
            r#"
            INSERT INTO menu_items (
                id, category_id, name, description, price_cents, currency, is_available,
                prep_time_minutes, image_url, spice_level, calories, is_chefs_special
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                category_id = excluded.category_id,
                name = excluded.name,
                description = excluded.description,
                price_cents = excluded.price_cents,
                currency = excluded.currency,
                is_available = excluded.is_available,
                prep_time_minutes = excluded.prep_time_minutes,
                image_url = excluded.image_url,
                spice_level = excluded.spice_level,
                calories = excluded.calories,
                is_chefs_special = excluded.is_chefs_special
            "#,
        )
        .bind(id)
        .bind(cat_id)
        .bind(name)
        .bind(desc)
        .bind(price)
        .bind(curr)
        .bind(avail)
        .bind(prep)
        .bind(img)
        .bind(spice)
        .bind(cal)
        .bind(special)
        .execute(pool)
        .await?;
    }

    // Set 2 items as unavailable for testing availability filters
    sqlx::query("UPDATE menu_items SET is_available = 0 WHERE id IN (4, 12)")
        .execute(pool)
        .await?;

    // 9. Item Dietary Tags Join Table
    // Tags: 1=vegan, 2=vegetarian, 3=gluten_free, 4=dairy_free, 5=halal, 6=nut_free
    let item_dietary_links = [
        (1, 2),
        (1, 3),
        (1, 6), // Palak Chaat: vegetarian, gluten_free, nut_free
        (2, 2),
        (2, 6), // Dahi Puri: vegetarian, nut_free
        (3, 1),
        (3, 2),
        (3, 3),
        (3, 4),
        (3, 6), // Idli: vegan, vegetarian, gluten_free, dairy_free, nut_free
        (4, 3),
        (4, 4),
        (4, 5),
        (4, 6), // Amritsari Fish: gluten_free, dairy_free, halal, nut_free
        (5, 1),
        (5, 2),
        (5, 3),
        (5, 4),
        (5, 6), // Bhindi: vegan, vegetarian, gluten_free, dairy_free, nut_free
        (6, 2),
        (6, 6), // Truffle Galouti: vegetarian, nut_free
        (7, 2),
        (7, 3),
        (7, 6), // Paneer Tikka: vegetarian, gluten_free, nut_free
        (8, 2),
        (8, 3), // Malai Broccoli: vegetarian, gluten_free (contains almonds)
        (9, 3),
        (9, 4),
        (9, 5),
        (9, 6), // Bhatti Ka Murgh: gluten_free, dairy_free, halal, nut_free
        (10, 3),
        (10, 5), // Murgh Malai Kebab: gluten_free, halal (contains cashews & dairy)
        (11, 3),
        (11, 4),
        (11, 5),
        (11, 6), // Kakori: gluten_free, dairy_free, halal, nut_free
        (12, 3),
        (12, 4),
        (12, 5),
        (12, 6), // Tiger Prawns: gluten_free, dairy_free, halal, nut_free
        (13, 2),
        (13, 3),
        (13, 6), // Dal Makhani: vegetarian, gluten_free, nut_free
        (14, 2),
        (14, 3),
        (14, 6), // Paneer Lababdar: vegetarian, gluten_free, nut_free
        (15, 1),
        (15, 2),
        (15, 3),
        (15, 4),
        (15, 6), // Baingan: vegan, veg, gf, df, nut_free
        (16, 1),
        (16, 2),
        (16, 3),
        (16, 4),
        (16, 6), // Chole: vegan, veg, gf, df, nut_free
        (17, 3),
        (17, 5),
        (17, 6), // Butter Chicken: gf, halal, nut_free (contains dairy)
        (18, 3),
        (18, 4),
        (18, 5),
        (18, 6), // Rogan Josh: gf, df, halal, nut_free
        (19, 3),
        (19, 4),
        (19, 5),
        (19, 6), // Fish Moilee: gf, df, halal, nut_free
        (20, 1),
        (20, 2),
        (20, 3),
        (20, 4),
        (20, 6), // Niligiri Korma: vegan, veg, gf, df, nut_free
        (21, 2),
        (21, 3),
        (21, 6), // Veg Biryani: veg, gf, nut_free (contains ghee)
        (22, 5),
        (22, 6), // Murgh Biryani: halal, nut_free (contains dairy & gluten purdah)
        (23, 3),
        (23, 5),
        (23, 6), // Gosht Biryani: gf, halal, nut_free (contains ghee)
        (24, 1),
        (24, 2),
        (24, 3),
        (24, 4),
        (24, 6), // Jeera Rice: vegan, veg, gf, df, nut_free (prepared in oil)
        (25, 2),
        (25, 6), // Truffle Naan: veg, nut_free (contains gluten & dairy)
        (26, 2),
        (26, 6), // Laccha Paratha: veg, nut_free (contains gluten & dairy)
        (27, 1),
        (27, 2),
        (27, 4),
        (27, 6), // Tandoori Roti: vegan, veg, df, nut_free (contains gluten)
        (28, 2),
        (28, 6), // Gulab Jamun: veg, nut_free (contains dairy & gluten)
        (29, 2),
        (29, 3), // Pistachio Kulfi: veg, gf (contains dairy & tree_nuts)
        (30, 2),
        (30, 3),
        (30, 6), // Rasgulla Mousse: veg, gf, nut_free (contains dairy)
        (31, 1),
        (31, 2),
        (31, 3),
        (31, 4),
        (31, 6), // Mango Kheer: vegan, veg, gf, df, nut_free
        (32, 1),
        (32, 2),
        (32, 3),
        (32, 4),
        (32, 6), // Kokum Cooler: vegan, veg, gf, df, nut_free
        (33, 2),
        (33, 3), // Thandai: veg, gf (contains dairy & tree_nuts)
        (34, 1),
        (34, 2),
        (34, 3),
        (34, 4),
        (34, 6), // Darjeeling Tea: vegan, veg, gf, df, nut_free
        (35, 2),
        (35, 3),
        (35, 6), // Karak Chai: veg, gf, nut_free (contains dairy)
    ];

    for (item_id, tag_id) in item_dietary_links {
        sqlx::query(
            "INSERT OR IGNORE INTO menu_item_dietary_tags (menu_item_id, tag_id) VALUES (?, ?)",
        )
        .bind(item_id)
        .bind(tag_id)
        .execute(pool)
        .await?;
    }

    // 10. Item Allergens Join Table
    // Allergens: 1=peanuts, 2=tree_nuts, 3=dairy, 4=gluten, 5=soy, 6=sesame, 7=mustard, 8=shellfish, 9=egg, 10=fish
    let item_allergen_links = [
        (1, 3), // Palak Chaat: dairy (spiced yogurt)
        (2, 3),
        (2, 4), // Dahi Puri: dairy, gluten
        // (3) Idli: no allergens
        (4, 10), // Fish Fingers: fish
        // (5) Bhindi: no allergens
        (6, 3),
        (6, 4), // Truffle Galouti: dairy, gluten
        (7, 3), // Paneer Tikka: dairy
        (8, 2),
        (8, 3), // Malai Broccoli: tree_nuts (almonds), dairy
        // (9) Bhatti Ka Murgh: no allergens
        (10, 2),
        (10, 3), // Murgh Malai: tree_nuts (cashews), dairy
        // (11) Kakori: no allergens
        (12, 8), // Tiger Prawns: shellfish
        (13, 3), // Dal Makhani: dairy (butter, cream)
        (14, 3), // Paneer Lababdar: dairy
        // (15) Baingan: no allergens
        // (16) Chole: no allergens
        (17, 3), // Butter Chicken: dairy
        // (18) Rogan Josh: no allergens
        (19, 10), // Fish Moilee: fish
        // (20) Niligiri Korma: no allergens
        (21, 3), // Veg Biryani: dairy (ghee)
        (22, 3),
        (22, 4), // Murgh Biryani: dairy (ghee), gluten (dough purdah)
        (23, 3), // Gosht Biryani: dairy (ghee)
        // (24) Jeera Rice: no allergens
        (25, 3),
        (25, 4), // Truffle Naan: dairy, gluten
        (26, 3),
        (26, 4), // Laccha Paratha: dairy, gluten
        (27, 4), // Tandoori Roti: gluten
        (28, 3),
        (28, 4), // Gulab Jamun: dairy, gluten
        (29, 2),
        (29, 3), // Pistachio Kulfi: tree_nuts, dairy
        (30, 3),
        (30, 5), // Rasgulla Mousse: dairy, soy (chocolate lecithin)
        // (31) Mango Kheer: no allergens
        // (32) Kokum Cooler: no allergens
        (33, 2),
        (33, 3), // Thandai: tree_nuts, dairy
        // (34) Darjeeling Tea: no allergens
        (35, 3), // Karak Chai: dairy
    ];

    for (item_id, allergen_id) in item_allergen_links {
        sqlx::query(
            "INSERT OR IGNORE INTO menu_item_allergens (menu_item_id, allergen_id) VALUES (?, ?)",
        )
        .bind(item_id)
        .bind(allergen_id)
        .execute(pool)
        .await?;
    }

    // 11. Customers (5 demo customers)
    let customers_data = [
        (
            1,
            "Rohan Sharma",
            Some("+91-555-0101"),
            Some("rohan.sharma@example.com"),
        ),
        (
            2,
            "Priya Patel",
            Some("+91-555-0102"),
            Some("priya.patel@example.com"),
        ),
        (
            3,
            "Arjun Menon",
            Some("+91-555-0103"),
            Some("arjun.menon@example.com"),
        ),
        (
            4,
            "Ananya Deshmukh",
            Some("+91-555-0104"),
            Some("ananya.d@example.com"),
        ),
        (
            5,
            "Vikramaditya Rao",
            Some("+91-555-0105"),
            Some("vikram.rao@example.com"),
        ),
    ];

    for (id, name, phone, email) in customers_data {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO customers (id, display_name, phone, email, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(phone)
        .bind(email)
        .bind(&now_iso)
        .execute(pool)
        .await?;
    }

    // 12. Customer Preferences (Preferences for session memory)
    let preferences_data = [
        (
            1,
            1,
            PreferenceKind::Allergy,
            "peanuts",
            PreferenceSource::Voice,
        ),
        (
            2,
            1,
            PreferenceKind::Dietary,
            "vegetarian",
            PreferenceSource::Explicit,
        ),
        (
            3,
            1,
            PreferenceKind::SeatPref,
            "patio",
            PreferenceSource::History,
        ),
        (
            4,
            1,
            PreferenceKind::FavoriteItem,
            "Paneer Tikka Angaare",
            PreferenceSource::History,
        ),
        (
            5,
            2,
            PreferenceKind::Dietary,
            "vegan",
            PreferenceSource::Explicit,
        ),
        (
            6,
            2,
            PreferenceKind::Allergy,
            "dairy",
            PreferenceSource::Explicit,
        ),
        (
            7,
            2,
            PreferenceKind::Note,
            "Prefers a quiet table away from the kitchen",
            PreferenceSource::Voice,
        ),
        (
            8,
            3,
            PreferenceKind::Dietary,
            "gluten_free",
            PreferenceSource::Explicit,
        ),
        (
            9,
            3,
            PreferenceKind::FavoriteItem,
            "Dum Gosht Biryani",
            PreferenceSource::History,
        ),
        (
            10,
            4,
            PreferenceKind::Dietary,
            "halal",
            PreferenceSource::Explicit,
        ),
        (
            11,
            4,
            PreferenceKind::SeatPref,
            "booth",
            PreferenceSource::Voice,
        ),
        (
            12,
            5,
            PreferenceKind::Allergy,
            "shellfish",
            PreferenceSource::Explicit,
        ),
        (
            13,
            5,
            PreferenceKind::Note,
            "Regularly celebrates anniversaries and birthdays here",
            PreferenceSource::Voice,
        ),
    ];

    for (id, cust_id, kind, val, src) in preferences_data {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO customer_preferences (id, customer_id, kind, value, source, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(cust_id)
        .bind(kind)
        .bind(val)
        .bind(src)
        .bind(&now_iso)
        .execute(pool)
        .await?;
    }

    // 13. Bookings (Historical completed + future busy slots)
    // Past booking 1: Rohan (Completed, 7 days ago)
    let past_7d_start = (now - Duration::days(7)).to_rfc3339();
    let past_7d_end = (now - Duration::days(7) + Duration::minutes(90)).to_rfc3339();
    // Past booking 2: Priya (Completed, 3 days ago)
    let past_3d_start = (now - Duration::days(3)).to_rfc3339();
    let past_3d_end = (now - Duration::days(3) + Duration::minutes(90)).to_rfc3339();

    // Future booking: Fully booked Friday 19:30 on Table 1, 2, 3, 4, 7 for conflict testing
    let future_fri_start = (now + Duration::days(4))
        .date_naive()
        .and_hms_opt(19, 30, 0)
        .unwrap()
        .and_utc()
        .to_rfc3339();
    let future_fri_end = (now + Duration::days(4))
        .date_naive()
        .and_hms_opt(21, 0, 0)
        .unwrap()
        .and_utc()
        .to_rfc3339();

    let bookings_data = [
        (
            "b0000000-0000-0000-0000-000000000001",
            "TT-8101",
            Some(1),
            "Rohan Sharma",
            2,
            past_7d_start,
            past_7d_end,
            BookingStatus::Completed,
            Some(7), // P01
            Some("Quiet corner on patio"),
            DepositStatus::None,
            0,
        ),
        (
            "b0000000-0000-0000-0000-000000000002",
            "TT-8102",
            Some(2),
            "Priya Patel",
            2,
            past_3d_start,
            past_3d_end,
            BookingStatus::Completed,
            Some(1), // T01
            Some("Strict vegan dining"),
            DepositStatus::None,
            0,
        ),
        (
            "b0000000-0000-0000-0000-000000000003",
            "TT-9201",
            Some(3),
            "Arjun Menon",
            4,
            future_fri_start.clone(),
            future_fri_end.clone(),
            BookingStatus::Confirmed,
            Some(3), // T03
            Some("Family dinner celebration"),
            DepositStatus::None,
            0,
        ),
        (
            "b0000000-0000-0000-0000-000000000004",
            "TT-9202",
            Some(4),
            "Ananya Deshmukh",
            4,
            future_fri_start.clone(),
            future_fri_end.clone(),
            BookingStatus::Confirmed,
            Some(4), // T04
            Some("Window booth requested"),
            DepositStatus::None,
            0,
        ),
        (
            "b0000000-0000-0000-0000-000000000005",
            "TT-9203",
            Some(5),
            "Vikramaditya Rao",
            6,
            future_fri_start,
            future_fri_end,
            BookingStatus::Confirmed,
            Some(5), // T05
            Some("Pre-paid group dining deposit"),
            DepositStatus::Paid,
            300000,
        ),
    ];

    for (b_id, code, cust, name, party, start, end, status, table, reqs, dep_stat, dep_amt) in
        bookings_data
    {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO bookings (
                id, confirmation_code, customer_id, guest_name, party_size,
                start_at, end_at, status, table_id, special_requests,
                deposit_status, deposit_amount_cents, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(b_id)
        .bind(code)
        .bind(cust)
        .bind(name)
        .bind(party)
        .bind(&start)
        .bind(&end)
        .bind(status)
        .bind(table)
        .bind(reqs)
        .bind(dep_stat)
        .bind(dep_amt)
        .bind(&now_iso)
        .bind(&now_iso)
        .execute(pool)
        .await?;
    }

    // 14. Orders (2 historical completed orders)
    #[allow(clippy::type_complexity)]
    let orders_data: [(
        &str,
        Option<i64>,
        Option<&str>,
        OrderType,
        OrderStatus,
        Option<&str>,
        i64,
        i64,
        i64,
        i64,
        &str,
        Option<&str>,
    ); 2] = [
        (
            "o0000000-0000-0000-0000-000000000001",
            Some(1),
            Some("b0000000-0000-0000-0000-000000000001"),
            OrderType::DineIn,
            OrderStatus::Completed,
            None,
            128000, // subtotal
            6400,   // tax 5%
            15000,  // tip
            149400, // total
            "INR",
            Some("Allergies: no peanuts"),
        ),
        (
            "o0000000-0000-0000-0000-000000000002",
            Some(2),
            Some("b0000000-0000-0000-0000-000000000002"),
            OrderType::DineIn,
            OrderStatus::Completed,
            None,
            97000,
            4850,
            10000,
            111850,
            "INR",
            Some("Strictly vegan preparation"),
        ),
    ];

    for (o_id, cust, b_id, o_type, status, pickup, sub, tax, tip, tot, curr, notes) in orders_data {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO orders (
                id, customer_id, booking_id, order_type, status, pickup_time,
                subtotal_cents, tax_cents, tip_cents, total_cents, currency,
                notes, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(o_id)
        .bind(cust)
        .bind(b_id)
        .bind(o_type)
        .bind(status)
        .bind(pickup)
        .bind(sub)
        .bind(tax)
        .bind(tip)
        .bind(tot)
        .bind(curr)
        .bind(notes)
        .bind(&now_iso)
        .bind(&now_iso)
        .execute(pool)
        .await?;
    }

    // 15. Order Items
    let order_items_data = [
        // Order 1 (Rohan)
        (
            1,
            "o0000000-0000-0000-0000-000000000001",
            1,
            1,
            38000,
            Some("Extra crispy"),
        ),
        (
            2,
            "o0000000-0000-0000-0000-000000000001",
            7,
            1,
            48000,
            Some("Mint chutney on side"),
        ),
        (
            3,
            "o0000000-0000-0000-0000-000000000001",
            13,
            1,
            42000,
            None,
        ),
        // Order 2 (Priya)
        (4, "o0000000-0000-0000-0000-000000000002", 3, 1, 29000, None),
        (
            5,
            "o0000000-0000-0000-0000-000000000002",
            15,
            1,
            39000,
            None,
        ),
        (
            6,
            "o0000000-0000-0000-0000-000000000002",
            31,
            1,
            29000,
            None,
        ),
    ];

    for (id, o_id, item_id, qty, unit_price, notes) in order_items_data {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO order_items (id, order_id, menu_item_id, quantity, unit_price_cents, notes)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(o_id)
        .bind(item_id)
        .bind(qty)
        .bind(unit_price)
        .bind(notes)
        .execute(pool)
        .await?;
    }

    // 16. Payments (Simulated Ledger)
    let payments_data = [
        (
            "p0000000-0000-0000-0000-000000000001",
            Some("b0000000-0000-0000-0000-000000000005"),
            None,
            300000,
            "INR",
            PaymentStatus::Captured,
            "sim_pay_dep_9f8a12",
        ),
        (
            "p0000000-0000-0000-0000-000000000002",
            None,
            Some("o0000000-0000-0000-0000-000000000001"),
            149400,
            "INR",
            PaymentStatus::Captured,
            "sim_pay_ord_1c4b77",
        ),
        (
            "p0000000-0000-0000-0000-000000000003",
            None,
            Some("o0000000-0000-0000-0000-000000000002"),
            111850,
            "INR",
            PaymentStatus::Captured,
            "sim_pay_ord_2e9d33",
        ),
    ];

    for (p_id, b_id, o_id, amt, curr, status, prov_ref) in payments_data {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO payments (id, booking_id, order_id, amount_cents, currency, status, provider_reference, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(p_id)
        .bind(b_id)
        .bind(o_id)
        .bind(amt)
        .bind(curr)
        .bind(status)
        .bind(prov_ref)
        .bind(&now_iso)
        .execute(pool)
        .await?;
    }

    // 17. Audit Events (Deterministic single row)
    sqlx::query(
        r#"
        INSERT INTO audit_events (id, entity, entity_id, action, actor, payload_json, created_at)
        VALUES (1, 'database', '1', 'seed_populated', ?, '{"generator": "seed_database"}', ?)
        ON CONFLICT (id) DO UPDATE SET created_at = excluded.created_at
        "#,
    )
    .bind(Actor::Seed)
    .bind(&now_iso)
    .execute(pool)
    .await?;

    Ok(())
}
