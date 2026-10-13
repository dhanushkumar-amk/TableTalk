use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

// --- Domain Enums for Input & Filtering ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DietaryTagFilter {
    Vegan,
    Vegetarian,
    GlutenFree,
    DairyFree,
    Halal,
    NutFree,
}

impl DietaryTagFilter {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::Vegan => "vegan",
            Self::Vegetarian => "vegetarian",
            Self::GlutenFree => "gluten_free",
            Self::DairyFree => "dairy_free",
            Self::Halal => "halal",
            Self::NutFree => "nut_free",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "vegan" => Some(Self::Vegan),
            "vegetarian" => Some(Self::Vegetarian),
            "gluten_free" | "gluten-free" => Some(Self::GlutenFree),
            "dairy_free" | "dairy-free" => Some(Self::DairyFree),
            "halal" => Some(Self::Halal),
            "nut_free" | "nut-free" => Some(Self::NutFree),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AllergenFilter {
    Peanuts,
    TreeNuts,
    Dairy,
    Gluten,
    Soy,
    Sesame,
    Mustard,
    Shellfish,
    Egg,
    Fish,
}

impl AllergenFilter {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::Peanuts => "peanuts",
            Self::TreeNuts => "tree_nuts",
            Self::Dairy => "dairy",
            Self::Gluten => "gluten",
            Self::Soy => "soy",
            Self::Sesame => "sesame",
            Self::Mustard => "mustard",
            Self::Shellfish => "shellfish",
            Self::Egg => "egg",
            Self::Fish => "fish",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "peanuts" | "peanut" => Some(Self::Peanuts),
            "tree_nuts" | "tree nuts" | "treenuts" | "tree nut" | "nuts" => Some(Self::TreeNuts),
            "dairy" | "milk" => Some(Self::Dairy),
            "gluten" | "wheat" => Some(Self::Gluten),
            "soy" | "soya" => Some(Self::Soy),
            "sesame" => Some(Self::Sesame),
            "mustard" => Some(Self::Mustard),
            "shellfish" | "crustacean" => Some(Self::Shellfish),
            "egg" | "eggs" => Some(Self::Egg),
            "fish" => Some(Self::Fish),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortOption {
    #[default]
    Relevance,
    PriceAsc,
    PriceDesc,
    Popular,
}

// --- Tool Output Models ---

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PriceDisplay {
    /// Price in minor currency units (paise)
    pub amount_minor: i64,
    /// ISO currency code (e.g. INR)
    pub currency: String,
    /// Human-readable formatted price string (e.g. ₹380)
    pub display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct MenuItemCard {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub category: String,
    pub price: PriceDisplay,
    pub dietary_tags: Vec<String>,
    pub allergens: Vec<String>,
    pub spice_level: i64,
    pub calories: Option<i64>,
    pub prep_time_minutes: i64,
    pub available: bool,
    pub chefs_special: bool,
    pub image_url: Option<String>,
    pub match_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AppliedFilters {
    pub query: Option<String>,
    pub category: Option<String>,
    pub dietary: Vec<DietaryTagFilter>,
    pub exclude_allergens: Vec<AllergenFilter>,
    pub max_price_inr: Option<f64>,
    pub include_unavailable: bool,
    pub sort: SortOption,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SearchMenuOutput {
    /// Human-readable echo of interpreted filters
    pub query_summary: String,
    /// Normalized filter values applied in database query
    pub applied_filters: AppliedFilters,
    /// Total number of items matching filters before pagination
    pub total_matches: usize,
    /// Number of items returned in this response
    pub returned: usize,
    /// Menu items matching criteria
    pub items: Vec<MenuItemCard>,
    /// Concrete suggestions if zero or few matches found
    pub suggestions: Vec<String>,
    /// Standard allergen handling notice for kitchen operations
    pub allergen_notice: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_applied: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needs_reconfirmation: Vec<String>,
}

pub const ALLERGEN_NOTICE: &str =
    "Kitchen handles common allergens including dairy, peanuts, tree nuts, and gluten. Please inform restaurant staff of any severe allergies before ordering.";

// --- Search Query Specification ---

#[derive(Debug, Clone, Default)]
pub struct MenuSearchQuery {
    pub raw_query: Option<String>,
    pub normalized_query: Option<String>,
    pub category: Option<String>,
    pub dietary: Vec<DietaryTagFilter>,
    pub exclude_allergens: Vec<AllergenFilter>,
    pub max_price_cents: Option<i64>,
    pub min_spice_level: Option<i64>,
    pub max_spice_level: Option<i64>,
    pub include_unavailable: bool,
    pub sort: SortOption,
    pub limit: usize,
}

// --- Query Normalization & Synonyms ---

/// Normalize a text query by lowercasing, stripping special punctuation, and trimming whitespace.
pub fn normalize_query_text(text: &str) -> String {
    let mut cleaned = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch.is_whitespace() {
            cleaned.push(ch.to_ascii_lowercase());
        } else {
            cleaned.push(' ');
        }
    }
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Strip common plural suffixes for naive stem matching in Indian cuisine terms.
pub fn strip_plural(word: &str) -> String {
    if word.ends_with("ies") && word.len() > 4 {
        return format!("{}y", &word[..word.len() - 3]);
    }
    if word.ends_with("es")
        && word.len() > 4
        && (word.ends_with("shes")
            || word.ends_with("ches")
            || word.ends_with("xes")
            || word.ends_with("sses"))
    {
        return word[..word.len() - 2].to_string();
    }
    if word.ends_with('s') && !word.ends_with("ss") && !word.ends_with("us") && word.len() > 3 {
        return word[..word.len() - 1].to_string();
    }
    word.to_string()
}

/// Inspect query string for documented domain synonyms and extract filters.
/// Supported synonyms:
/// - "veg" -> vegetarian
/// - "nut-free", "nut free" -> nut_free tag + exclude peanuts and tree_nuts
/// - "no dairy", "dairy-free", "dairy free" -> dairy_free tag + exclude dairy
/// - "spicy" -> spice_level >= 2
/// - "mild" -> spice_level == 0
pub fn apply_synonyms(
    raw_query: &str,
    dietary: &mut Vec<DietaryTagFilter>,
    exclude_allergens: &mut Vec<AllergenFilter>,
    min_spice: &mut Option<i64>,
    max_spice: &mut Option<i64>,
) {
    let lower = raw_query.to_ascii_lowercase();

    if (lower.split_whitespace().any(|w| w == "veg") || lower == "veg")
        && !dietary.contains(&DietaryTagFilter::Vegetarian)
        && !dietary.contains(&DietaryTagFilter::Vegan)
    {
        dietary.push(DietaryTagFilter::Vegetarian);
    }

    if (lower.contains("nut-free") || lower.contains("nut free"))
        && !dietary.contains(&DietaryTagFilter::NutFree)
    {
        dietary.push(DietaryTagFilter::NutFree);
        if !exclude_allergens.contains(&AllergenFilter::Peanuts) {
            exclude_allergens.push(AllergenFilter::Peanuts);
        }
        if !exclude_allergens.contains(&AllergenFilter::TreeNuts) {
            exclude_allergens.push(AllergenFilter::TreeNuts);
        }
    }

    if lower.contains("no dairy") || lower.contains("dairy-free") || lower.contains("dairy free") {
        if !dietary.contains(&DietaryTagFilter::DairyFree) {
            dietary.push(DietaryTagFilter::DairyFree);
        }
        if !exclude_allergens.contains(&AllergenFilter::Dairy) {
            exclude_allergens.push(AllergenFilter::Dairy);
        }
    }

    if (lower.contains("spicy") || lower.contains("hot")) && min_spice.is_none() {
        *min_spice = Some(2);
    }

    if lower.contains("mild") && max_spice.is_none() {
        *max_spice = Some(0);
    }
}

/// Sanitize text for FTS5 queries by removing query syntax operators.
pub fn sanitize_fts_query(text: &str) -> String {
    let tokens: Vec<String> = text
        .split_whitespace()
        .filter_map(|token| {
            let clean: String = token.chars().filter(|c| c.is_alphanumeric()).collect();
            let upper = clean.to_ascii_uppercase();
            if clean.is_empty()
                || upper == "AND"
                || upper == "OR"
                || upper == "NOT"
                || upper == "NEAR"
            {
                None
            } else {
                Some(format!("{}*", clean))
            }
        })
        .collect();

    tokens.join(" ")
}

// --- Spoken Currency & Price Formatting ---

/// Convert paise (minor unit) into spoken English words for Alexa voice readout.
pub fn paise_to_spoken_rupees(amount_paise: i64) -> String {
    let rupees = amount_paise / 100;
    let paise = amount_paise % 100;

    let rupee_text = number_to_words(rupees);
    let unit = if rupees == 1 { "rupee" } else { "rupees" };

    if paise == 0 {
        format!("{} {}", rupee_text, unit)
    } else {
        let paise_text = number_to_words(paise);
        format!("{} {} and {} paise", rupee_text, unit, paise_text)
    }
}

fn number_to_words(n: i64) -> String {
    if n == 0 {
        return "zero".to_string();
    }
    if n < 0 {
        return format!("negative {}", number_to_words(-n));
    }

    let units = [
        "",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];

    let tens = [
        "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];

    if n < 20 {
        return units[n as usize].to_string();
    }

    if n < 100 {
        let t = (n / 10) as usize;
        let u = (n % 10) as usize;
        return if u == 0 {
            tens[t].to_string()
        } else {
            format!("{} {}", tens[t], units[u])
        };
    }

    if n < 1000 {
        let h = (n / 100) as usize;
        let rem = n % 100;
        return if rem == 0 {
            format!("{} hundred", units[h])
        } else {
            format!("{} hundred {}", units[h], number_to_words(rem))
        };
    }

    let k = n / 1000;
    let rem = n % 1000;
    if rem == 0 {
        format!("{} thousand", number_to_words(k))
    } else {
        format!("{} thousand {}", number_to_words(k), number_to_words(rem))
    }
}

// --- Voice Text Generation ---

/// Generate voice text for speech synthesis (Alexa+).
/// Must be 1-3 short sentences, natural spoken English, no markdown, no emoji, no tables, no raw IDs.
/// Employs strict allergen phrasing ("listed as not containing X", NEVER "safe" or "guaranteed").
pub fn generate_voice_text(
    total_matches: usize,
    items: &[MenuItemCard],
    excluded_allergens: &[AllergenFilter],
    suggestions: &[String],
) -> String {
    if total_matches == 0 {
        if let Some(first_suggestion) = suggestions.first() {
            return format!(
                "I couldn't find any items matching all your filters. {}. Would you like to try that?",
                first_suggestion
            );
        } else {
            return "I couldn't find any dishes matching your request. Would you like to hear our chef's specials or browse by category?".to_string();
        }
    }

    let mut sentences = Vec::new();

    // Sentence 1 & 2: Count and top 2-3 items with spoken prices
    let top_items: Vec<_> = items.iter().take(3).collect();
    let spoken_items: Vec<String> = top_items
        .iter()
        .map(|it| {
            let spoken_price = paise_to_spoken_rupees(it.price.amount_minor);
            format!("{} for {}", it.name, spoken_price)
        })
        .collect();

    if total_matches == 1 {
        sentences.push(format!("I found 1 dish: {}.", spoken_items[0]));
    } else if total_matches <= 3 {
        sentences.push(format!(
            "I found {} dishes: {}.",
            total_matches,
            spoken_items.join(", and ")
        ));
    } else {
        sentences.push(format!(
            "I found {} matching dishes. Top choices are {}.",
            total_matches,
            spoken_items.join(", ")
        ));
    }

    // Sentence 2/3: Allergen disclaimer or prompt to narrow down
    if !excluded_allergens.is_empty() {
        let allergen_names: Vec<&str> = excluded_allergens.iter().map(|a| a.as_db_str()).collect();
        let allergens_str = allergen_names.join(", ");
        sentences.push(format!(
            "These items are listed as not containing {}. Please confirm with restaurant staff before ordering.",
            allergens_str
        ));
    } else if total_matches > 3 {
        sentences.push("Would you like to hear more or narrow down by category?".to_string());
    }

    sentences.join(" ")
}

// --- DB Search Engine ---

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct RawSearchResult {
    id: i64,
    category_id: i64,
    name: String,
    description: String,
    price_cents: i64,
    currency: String,
    is_available: bool,
    prep_time_minutes: i64,
    image_url: Option<String>,
    spice_level: i64,
    calories: Option<i64>,
    is_chefs_special: bool,
    allergens_verified: bool,
    category_name: String,
}

pub async fn execute_menu_search(
    pool: &SqlitePool,
    query: MenuSearchQuery,
) -> Result<SearchMenuOutput, sqlx::Error> {
    // 1. Build and execute search query
    let (items_raw, total_matches) = run_search_query(pool, &query).await?;

    // 2. Fetch dietary tags and allergens for returned items
    let mut item_cards = Vec::with_capacity(items_raw.len());
    for raw in items_raw {
        let dietary_tags: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT dt.name
            FROM dietary_tags dt
            JOIN menu_item_dietary_tags midt ON midt.tag_id = dt.id
            WHERE midt.menu_item_id = ?
            ORDER BY dt.name ASC
            "#,
        )
        .bind(raw.id)
        .fetch_all(pool)
        .await?;

        let allergens: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT a.name
            FROM allergens a
            JOIN menu_item_allergens mia ON mia.allergen_id = a.id
            WHERE mia.menu_item_id = ?
            ORDER BY a.name ASC
            "#,
        )
        .bind(raw.id)
        .fetch_all(pool)
        .await?;

        // Construct match reason (strictly allergen-safe wording: "listed as not containing X")
        let mut reasons = Vec::new();
        if !query.dietary.is_empty() {
            reasons.push(format!(
                "tagged {}",
                query
                    .dietary
                    .iter()
                    .map(|d| d.as_db_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !query.exclude_allergens.is_empty() {
            reasons.push(format!(
                "listed as not containing {}",
                query
                    .exclude_allergens
                    .iter()
                    .map(|a| a.as_db_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if raw.is_chefs_special {
            reasons.push("chef's signature dish".to_string());
        }
        if reasons.is_empty() {
            reasons.push("matched menu search".to_string());
        }
        let match_reason = reasons.join("; ");

        let display_price = format!("₹{}", raw.price_cents / 100);

        item_cards.push(MenuItemCard {
            id: raw.id,
            name: raw.name,
            description: raw.description,
            category: raw.category_name,
            price: PriceDisplay {
                amount_minor: raw.price_cents,
                currency: raw.currency,
                display: display_price,
            },
            dietary_tags,
            allergens,
            spice_level: raw.spice_level,
            calories: raw.calories,
            prep_time_minutes: raw.prep_time_minutes,
            available: raw.is_available,
            chefs_special: raw.is_chefs_special,
            image_url: raw.image_url,
            match_reason,
        });
    }

    // 3. Truthfully compute suggestions if empty or low matches
    let mut suggestions = Vec::new();
    if total_matches == 0 {
        suggestions = compute_suggestions(pool, &query).await?;
    }

    // 4. Construct human-readable query summary
    let query_summary = build_query_summary(&query, total_matches);

    let max_price_inr = query.max_price_cents.map(|c| c as f64 / 100.0);

    let applied_filters = AppliedFilters {
        query: query.raw_query,
        category: query.category,
        dietary: query.dietary,
        exclude_allergens: query.exclude_allergens,
        max_price_inr,
        include_unavailable: query.include_unavailable,
        sort: query.sort,
        limit: query.limit,
    };

    let returned = item_cards.len();

    Ok(SearchMenuOutput {
        query_summary,
        applied_filters,
        total_matches,
        returned,
        items: item_cards,
        suggestions,
        allergen_notice: ALLERGEN_NOTICE.to_string(),
        profile_applied: None,
        needs_reconfirmation: vec![],
    })
}

async fn run_search_query(
    pool: &SqlitePool,
    query: &MenuSearchQuery,
) -> Result<(Vec<RawSearchResult>, usize), sqlx::Error> {
    // 1. Try with FTS if text query present
    if let Some(ref text) = query.normalized_query {
        let fts_pattern = sanitize_fts_query(text);
        if !fts_pattern.is_empty() {
            match build_and_exec_query(pool, query, Some(&fts_pattern), false).await {
                Ok(res) => {
                    if res.1 > 0 {
                        return Ok(res);
                    }
                }
                Err(e) => {
                    tracing::warn!("FTS search failed, falling back to LIKE: {}", e);
                }
            }
        }
    }

    // 2. Standard execution (or LIKE fallback if query present)
    build_and_exec_query(pool, query, None, query.normalized_query.is_some()).await
}

async fn build_and_exec_query(
    pool: &SqlitePool,
    query: &MenuSearchQuery,
    fts_pattern: Option<&str>,
    use_like_fallback: bool,
) -> Result<(Vec<RawSearchResult>, usize), sqlx::Error> {
    // Count query
    let mut count_qb: QueryBuilder<Sqlite> = QueryBuilder::new("SELECT COUNT(*) FROM menu_items m JOIN menu_categories c ON c.id = m.category_id WHERE 1=1");
    apply_where_clauses(&mut count_qb, query, fts_pattern, use_like_fallback);
    let total_matches: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    // Items query
    let mut items_qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        r#"
        SELECT 
            m.id, m.category_id, m.name, m.description, m.price_cents, m.currency,
            m.is_available, m.prep_time_minutes, m.image_url, m.spice_level,
            m.calories, m.is_chefs_special, m.allergens_verified,
            c.name AS category_name
        FROM menu_items m
        JOIN menu_categories c ON c.id = m.category_id
        WHERE 1=1
        "#,
    );
    apply_where_clauses(&mut items_qb, query, fts_pattern, use_like_fallback);

    // Sorting
    match query.sort {
        SortOption::PriceAsc => {
            items_qb.push(" ORDER BY m.is_available DESC, m.price_cents ASC, m.id ASC");
        }
        SortOption::PriceDesc => {
            items_qb.push(" ORDER BY m.is_available DESC, m.price_cents DESC, m.id ASC");
        }
        SortOption::Popular => {
            items_qb.push(
                " ORDER BY m.is_available DESC, m.is_chefs_special DESC, m.price_cents ASC, m.id ASC",
            );
        }
        SortOption::Relevance => {
            items_qb.push(
                " ORDER BY m.is_available DESC, m.is_chefs_special DESC, m.price_cents ASC, m.id ASC",
            );
        }
    }

    items_qb.push(" LIMIT ");
    items_qb.push_bind(query.limit as i64);

    let items = items_qb
        .build_query_as::<RawSearchResult>()
        .fetch_all(pool)
        .await?;

    Ok((items, total_matches as usize))
}

fn apply_where_clauses<'a>(
    qb: &mut QueryBuilder<'a, Sqlite>,
    query: &'a MenuSearchQuery,
    fts_pattern: Option<&'a str>,
    use_like_fallback: bool,
) {
    if !query.include_unavailable {
        qb.push(" AND m.is_available = 1");
    }

    if let Some(ref cat) = query.category {
        qb.push(" AND LOWER(c.name) = LOWER(");
        qb.push_bind(cat);
        qb.push(")");
    }

    if let Some(max_cents) = query.max_price_cents {
        qb.push(" AND m.price_cents <= ");
        qb.push_bind(max_cents);
    }

    if let Some(min_s) = query.min_spice_level {
        qb.push(" AND m.spice_level >= ");
        qb.push_bind(min_s);
    }

    if let Some(max_s) = query.max_spice_level {
        qb.push(" AND m.spice_level <= ");
        qb.push_bind(max_s);
    }

    // Dietary tags (MUST have all requested tags)
    for tag in &query.dietary {
        qb.push(
            " AND m.id IN (SELECT midt.menu_item_id FROM menu_item_dietary_tags midt JOIN dietary_tags dt ON dt.id = midt.tag_id WHERE dt.name = ",
        );
        qb.push_bind(tag.as_db_str());
        qb.push(")");
    }

    // ALLERGEN SAFETY RULES (Hard filter in SQL, never post-hoc)
    if !query.exclude_allergens.is_empty() {
        // Items with unverified allergens are excluded
        qb.push(" AND m.allergens_verified = 1");

        // Items containing any excluded allergen are strictly excluded
        qb.push(
            " AND m.id NOT IN (SELECT mia.menu_item_id FROM menu_item_allergens mia JOIN allergens a ON a.id = mia.allergen_id WHERE a.name IN (",
        );
        let mut sep = qb.separated(", ");
        for allergen in &query.exclude_allergens {
            sep.push_bind(allergen.as_db_str());
        }
        qb.push("))");
    }

    // Free text matching
    if let Some(fts) = fts_pattern {
        qb.push(" AND m.id IN (SELECT item_id FROM menu_items_fts WHERE menu_items_fts MATCH ");
        qb.push_bind(fts);
        qb.push(")");
    } else if use_like_fallback {
        if let Some(ref q) = query.normalized_query {
            let like_term = format!("%{}%", q);
            qb.push(" AND (LOWER(m.name) LIKE ");
            qb.push_bind(like_term.clone());
            qb.push(" OR LOWER(m.description) LIKE ");
            qb.push_bind(like_term.clone());
            qb.push(" OR LOWER(c.name) LIKE ");
            qb.push_bind(like_term);
            qb.push(")");
        }
    }
}

/// Truthfully compute suggestions by relaxing non-safety filters.
/// Allergen exclusion filters are NEVER relaxed (safety invariant).
async fn compute_suggestions(
    pool: &SqlitePool,
    query: &MenuSearchQuery,
) -> Result<Vec<String>, sqlx::Error> {
    let mut suggestions = Vec::new();

    // 1. Try relaxing dietary tags one by one
    if !query.dietary.is_empty() {
        for tag in &query.dietary {
            let mut relaxed = query.clone();
            relaxed.dietary.retain(|t| t != tag);
            let (_, count) = run_search_query(pool, &relaxed).await?;
            if count > 0 {
                suggestions.push(format!(
                    "Removing the '{}' dietary filter would match {} dishes",
                    tag.as_db_str(),
                    count
                ));
            }
        }
    }

    // 2. Try relaxing max_price
    if let Some(max_cents) = query.max_price_cents {
        let mut relaxed = query.clone();
        relaxed.max_price_cents = None;
        let (_, count) = run_search_query(pool, &relaxed).await?;
        if count > 0 {
            suggestions.push(format!(
                "Removing the budget limit of ₹{} would match {} dishes",
                max_cents / 100,
                count
            ));
        }
    }

    // 3. Try relaxing category
    if let Some(ref cat) = query.category {
        let mut relaxed = query.clone();
        relaxed.category = None;
        let (_, count) = run_search_query(pool, &relaxed).await?;
        if count > 0 {
            suggestions.push(format!(
                "Expanding beyond category '{}' would match {} dishes",
                cat, count
            ));
        }
    }

    // 4. Try relaxing query text
    if query.normalized_query.is_some() {
        let mut relaxed = query.clone();
        relaxed.normalized_query = None;
        relaxed.raw_query = None;
        let (_, count) = run_search_query(pool, &relaxed).await?;
        if count > 0 {
            suggestions.push(format!(
                "Browsing without the specific search keyword would match {} dishes",
                count
            ));
        }
    }

    // 5. Try including unavailable items if unavailable was excluded
    if !query.include_unavailable {
        let mut relaxed = query.clone();
        relaxed.include_unavailable = true;
        let (_, count) = run_search_query(pool, &relaxed).await?;
        if count > 0 {
            suggestions.push(format!(
                "Including dishes that are temporarily out of stock would match {} items",
                count
            ));
        }
    }

    Ok(suggestions)
}

fn build_query_summary(query: &MenuSearchQuery, total: usize) -> String {
    let mut parts = Vec::new();

    if let Some(ref q) = query.raw_query {
        parts.push(format!("matching \"{}\"", q));
    }
    if let Some(ref cat) = query.category {
        parts.push(format!("in category \"{}\"", cat));
    }
    if !query.dietary.is_empty() {
        let tags: Vec<&str> = query.dietary.iter().map(|d| d.as_db_str()).collect();
        parts.push(format!("tagged {}", tags.join(", ")));
    }
    if !query.exclude_allergens.is_empty() {
        let allergens: Vec<&str> = query
            .exclude_allergens
            .iter()
            .map(|a| a.as_db_str())
            .collect();
        parts.push(format!("excluding {}", allergens.join(", ")));
    }
    if let Some(max_cents) = query.max_price_cents {
        parts.push(format!("under ₹{}", max_cents / 100));
    }

    if parts.is_empty() {
        format!("Found {} menu items", total)
    } else {
        format!("Found {} items {}", total, parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_query_text() {
        assert_eq!(
            normalize_query_text("   Crisp   Palak! Chaat? "),
            "crisp palak chaat"
        );
        assert_eq!(normalize_query_text("Butter-Chicken"), "butter chicken");
        assert_eq!(normalize_query_text("100% Vegan!"), "100 vegan");
    }

    #[test]
    fn test_strip_plural() {
        assert_eq!(strip_plural("curries"), "curry");
        assert_eq!(strip_plural("rotis"), "roti");
        assert_eq!(strip_plural("kebabs"), "kebab");
        assert_eq!(strip_plural("hummus"), "hummus");
        assert_eq!(strip_plural("tea"), "tea");
    }

    #[test]
    fn test_apply_synonyms() {
        // veg
        let mut dietary = Vec::new();
        let mut allergens = Vec::new();
        let mut min_spice = None;
        let mut max_spice = None;
        apply_synonyms(
            "show me veg starters",
            &mut dietary,
            &mut allergens,
            &mut min_spice,
            &mut max_spice,
        );
        assert!(dietary.contains(&DietaryTagFilter::Vegetarian));

        // nut-free
        let mut dietary = Vec::new();
        let mut allergens = Vec::new();
        let mut min_spice = None;
        let mut max_spice = None;
        apply_synonyms(
            "nut free dishes please",
            &mut dietary,
            &mut allergens,
            &mut min_spice,
            &mut max_spice,
        );
        assert!(dietary.contains(&DietaryTagFilter::NutFree));
        assert!(allergens.contains(&AllergenFilter::Peanuts));
        assert!(allergens.contains(&AllergenFilter::TreeNuts));

        // no dairy
        let mut dietary = Vec::new();
        let mut allergens = Vec::new();
        let mut min_spice = None;
        let mut max_spice = None;
        apply_synonyms(
            "no dairy curry",
            &mut dietary,
            &mut allergens,
            &mut min_spice,
            &mut max_spice,
        );
        assert!(dietary.contains(&DietaryTagFilter::DairyFree));
        assert!(allergens.contains(&AllergenFilter::Dairy));

        // spicy / mild
        let mut dietary = Vec::new();
        let mut allergens = Vec::new();
        let mut min_spice = None;
        let mut max_spice = None;
        apply_synonyms(
            "something spicy",
            &mut dietary,
            &mut allergens,
            &mut min_spice,
            &mut max_spice,
        );
        assert_eq!(min_spice, Some(2));

        let mut min_spice = None;
        let mut max_spice = None;
        apply_synonyms(
            "mild biryani",
            &mut dietary,
            &mut allergens,
            &mut min_spice,
            &mut max_spice,
        );
        assert_eq!(max_spice, Some(0));
    }

    #[test]
    fn test_sanitize_fts_query() {
        assert_eq!(sanitize_fts_query("palak OR chaat"), "palak* chaat*");
        assert_eq!(
            sanitize_fts_query("NOT butter* AND chicken"),
            "butter* chicken*"
        );
        assert_eq!(sanitize_fts_query(""), "");
    }

    #[test]
    fn test_paise_to_spoken_rupees() {
        assert_eq!(paise_to_spoken_rupees(100), "one rupee");
        assert_eq!(paise_to_spoken_rupees(500), "five rupees");
        assert_eq!(paise_to_spoken_rupees(9000), "ninety rupees");
        assert_eq!(paise_to_spoken_rupees(14000), "one hundred forty rupees");
        assert_eq!(paise_to_spoken_rupees(29000), "two hundred ninety rupees");
        assert_eq!(paise_to_spoken_rupees(32000), "three hundred twenty rupees");
        assert_eq!(paise_to_spoken_rupees(38000), "three hundred eighty rupees");
        assert_eq!(paise_to_spoken_rupees(68000), "six hundred eighty rupees");
        assert_eq!(paise_to_spoken_rupees(100000), "one thousand rupees");
        assert_eq!(
            paise_to_spoken_rupees(125050),
            "one thousand two hundred fifty rupees and fifty paise"
        );
    }

    #[test]
    fn test_generate_voice_text_single_and_multi() {
        let items = vec![MenuItemCard {
            id: 1,
            name: "Crisp Palak Chaat".to_string(),
            description: "Spinach chaat".to_string(),
            category: "Small Plates & Starters".to_string(),
            price: PriceDisplay {
                amount_minor: 38000,
                currency: "INR".to_string(),
                display: "₹380".to_string(),
            },
            dietary_tags: vec!["vegetarian".to_string()],
            allergens: vec![],
            spice_level: 1,
            calories: Some(310),
            prep_time_minutes: 12,
            available: true,
            chefs_special: true,
            image_url: None,
            match_reason: "matched menu search".to_string(),
        }];

        let voice = generate_voice_text(1, &items, &[], &[]);
        assert_eq!(
            voice,
            "I found 1 dish: Crisp Palak Chaat for three hundred eighty rupees."
        );

        let voice_allergen = generate_voice_text(1, &items, &[AllergenFilter::Peanuts], &[]);
        assert!(voice_allergen.contains("listed as not containing peanuts"));
        assert!(voice_allergen.contains("Please confirm with restaurant staff before ordering."));

        // Prohibited safety claims assertion
        let lower = voice_allergen.to_lowercase();
        assert!(!lower.contains("safe"), "Must never say 'safe'");
        assert!(!lower.contains("guaranteed"), "Must never say 'guaranteed'");
        assert!(
            !lower.contains("allergy-free"),
            "Must never say 'allergy-free'"
        );
    }

    #[test]
    fn test_generate_voice_text_empty_suggestions() {
        let suggestions =
            vec!["Removing the 'gluten_free' filter would match 8 dishes".to_string()];
        let voice = generate_voice_text(0, &[], &[], &suggestions);
        assert!(voice.contains("Removing the 'gluten_free' filter would match 8 dishes"));
        assert!(voice.contains("Would you like to try that?"));
    }

    #[test]
    fn test_allergen_notice_safety_vocabulary() {
        let notice_lower = ALLERGEN_NOTICE.to_lowercase();
        assert!(!notice_lower.contains("safe"));
        assert!(!notice_lower.contains("guaranteed"));
        assert!(!notice_lower.contains("allergy-free"));
    }
}
