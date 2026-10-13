use crate::menu_search::{
    apply_synonyms, execute_menu_search, generate_voice_text, normalize_query_text, AllergenFilter,
    DietaryTagFilter, MenuSearchQuery, SortOption,
};
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

pub use crate::menu_search::{
    AppliedFilters, MenuItemCard, PriceDisplay, SearchMenuOutput, ALLERGEN_NOTICE,
};

/// Input parameters for the `search_menu` MCP tool.
/// Designed for LLM agent callers with explicit domain validations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SearchMenuInput {
    /// Free text query for dish name, ingredient, cuisine style, or mood (e.g. 'biryani', 'spinach', 'spicy'). Maximum 200 characters.
    pub query: Option<String>,

    /// Category name to filter by (e.g. 'Small Plates & Starters', 'Tandoor & Charcoal Grills', 'Signature Curries & Mains').
    pub category: Option<String>,

    /// Dietary tags the item MUST have (e.g. ['vegan', 'gluten_free']). Only valid tags recognized by the restaurant are allowed.
    pub dietary: Option<Vec<DietaryTagFilter>>,

    /// Allergens the item MUST NOT contain (e.g. ['peanuts', 'dairy', 'gluten']). Hard filter in database.
    pub exclude_allergens: Option<Vec<AllergenFilter>>,

    /// Maximum price threshold in Indian Rupees (INR) (e.g. 500.0 for dishes up to ₹500).
    pub max_price: Option<f64>,

    /// Whether to include dishes that are temporarily out of stock. Defaults to false.
    pub include_unavailable: Option<bool>,

    /// Sort ordering: 'relevance' (default), 'price_asc', 'price_desc', or 'popular'.
    pub sort: Option<SortOption>,

    /// Maximum number of items to return (1-10, default 5). Kept short for voice assistant readability.
    pub limit: Option<usize>,

    /// Optional profile bearer token to automatically apply saved dietary and allergy preferences.
    pub profile_token: Option<String>,

    /// Whether to apply preferences from the profile associated with `profile_token`. Defaults to false.
    pub use_profile: Option<bool>,
}

/// Helper to produce a tool execution error (isError: true) allowing the LLM to self-correct.
fn tool_validation_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

/// Business logic for executing menu search with hard safety filtering and voice synthesis.
pub async fn handle_search_menu(pool: &SqlitePool, input: SearchMenuInput) -> CallToolResult {
    let clock = crate::availability::SystemClock;
    handle_search_menu_with_clock(pool, input, &clock).await
}

pub async fn handle_search_menu_with_clock(
    pool: &SqlitePool,
    input: SearchMenuInput,
    clock: &dyn crate::availability::Clock,
) -> CallToolResult {
    // 1. Validate inputs
    if let Some(ref q) = input.query {
        if q.chars().count() > 200 {
            return tool_validation_error(format!(
                "Invalid input: 'query' exceeds maximum length of 200 characters (provided {} characters).",
                q.chars().count()
            ));
        }
    }

    if let Some(l) = input.limit {
        if !(1..=10).contains(&l) {
            return tool_validation_error(format!(
                "Invalid input: 'limit' must be between 1 and 10 (received {}). Voice responses must remain concise.",
                l
            ));
        }
    }

    if let Some(p) = input.max_price {
        if p < 0.0 || p.is_nan() || p.is_infinite() {
            return tool_validation_error(
                "Invalid input: 'max_price' must be a non-negative finite number in Indian Rupees (INR).",
            );
        }
    }

    // Profile token resolution
    let use_profile_flag = input.use_profile.unwrap_or(false);
    let (profile_applied, needs_reconfirmation, profile_allergies, profile_dietary) = if let Some(
        token_raw,
    ) = input
        .profile_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let rate_limiter = crate::memory::get_token_rate_limiter();
        match crate::memory::resolve_profile_token(pool, token_raw, rate_limiter, clock).await {
            Ok(Some(customer)) => {
                if use_profile_flag {
                    let cust_id = customer.id;
                    let prefs: Vec<(String, String, Option<String>)> = sqlx::query_as(
                            "SELECT kind, value, confirmed_at FROM customer_preferences WHERE customer_id = ? AND kind IN ('allergy', 'dietary')"
                        )
                        .bind(cust_id)
                        .fetch_all(pool)
                        .await
                        .unwrap_or_default();

                    let mut reconf = Vec::new();
                    let reconf_cutoff = clock.now()
                        - chrono::Duration::days(crate::memory::ALLERGY_RECONFIRMATION_DAYS);
                    let mut allergies = Vec::new();
                    let mut dietary = Vec::new();

                    for (kind, value, confirmed_at) in prefs {
                        if kind == "allergy" {
                            let is_stale = match &confirmed_at {
                                Some(ts) => match chrono::DateTime::parse_from_rfc3339(ts) {
                                    Ok(dt) => dt.with_timezone(&chrono::Utc) < reconf_cutoff,
                                    Err(_) => true,
                                },
                                None => true,
                            };
                            if is_stale && !reconf.contains(&value) {
                                reconf.push(value.clone());
                            }
                            if let Some(filt) = AllergenFilter::from_db_str(&value) {
                                allergies.push(filt);
                            }
                        } else if kind == "dietary" {
                            if let Some(filt) = DietaryTagFilter::from_db_str(&value) {
                                dietary.push(filt);
                            }
                        }
                    }

                    (Some(true), reconf, allergies, dietary)
                } else {
                    (None, vec![], vec![], vec![])
                }
            }
            Ok(None) => {
                let output = SearchMenuOutput {
                    query_summary: "Search cancelled: profile not found.".to_string(),
                    applied_filters: AppliedFilters {
                        query: input.query,
                        category: input.category,
                        dietary: vec![],
                        exclude_allergens: vec![],
                        max_price_inr: None,
                        include_unavailable: false,
                        sort: SortOption::Relevance,
                        limit: 5,
                    },
                    total_matches: 0,
                    returned: 0,
                    items: vec![],
                    suggestions: vec![],
                    allergen_notice: ALLERGEN_NOTICE.to_string(),
                    profile_applied: Some(false),
                    needs_reconfirmation: vec![],
                };
                let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
                res.content = vec![ContentBlock::text(
                    "Profile not found. Stored dining preferences and allergies were not applied.",
                )];
                return res;
            }
            Err(e) => {
                return CallToolResult::error(vec![ContentBlock::text(format!(
                    "Database error resolving profile token: {}",
                    e
                ))]);
            }
        }
    } else {
        (None, vec![], vec![], vec![])
    };

    // 2. Prepare normalized search query
    let raw_query = input.query.clone();
    let normalized_query = input.query.as_deref().and_then(|q| {
        let norm = normalize_query_text(q);
        if norm.is_empty() {
            None
        } else {
            Some(norm)
        }
    });

    let mut dietary = input.dietary.unwrap_or_default();
    let mut exclude_allergens = input.exclude_allergens.unwrap_or_default();
    let mut min_spice_level = None;
    let mut max_spice_level = None;

    if let Some(ref raw) = raw_query {
        apply_synonyms(
            raw,
            &mut dietary,
            &mut exclude_allergens,
            &mut min_spice_level,
            &mut max_spice_level,
        );
    }

    // Merge profile dietary and allergies
    for d in profile_dietary {
        if !dietary.contains(&d) {
            dietary.push(d);
        }
    }
    for a in profile_allergies {
        if !exclude_allergens.contains(&a) {
            exclude_allergens.push(a);
        }
    }

    let max_price_cents = input.max_price.map(|p| (p * 100.0).round() as i64);
    let include_unavailable = input.include_unavailable.unwrap_or(false);
    let sort = input.sort.unwrap_or_default();
    let limit = input.limit.unwrap_or(5);

    let search_spec = MenuSearchQuery {
        raw_query,
        normalized_query,
        category: input.category,
        dietary,
        exclude_allergens,
        max_price_cents,
        min_spice_level,
        max_spice_level,
        include_unavailable,
        sort,
        limit,
    };

    // 3. Execute query
    let mut output = match execute_menu_search(pool, search_spec).await {
        Ok(out) => out,
        Err(err) => {
            tracing::error!("Database query failed during menu search: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal database error occurred while searching the menu. Please try again or ask restaurant staff.",
            )]);
        }
    };

    // 4. Generate voice text (spoken natural English with spoken prices and allergen phrasing)
    let mut voice_text = generate_voice_text(
        output.total_matches,
        &output.items,
        &output.applied_filters.exclude_allergens,
        &output.suggestions,
    );

    if let Some(applied) = profile_applied {
        output.profile_applied = Some(applied);
        output.needs_reconfirmation = needs_reconfirmation.clone();
        if applied {
            voice_text = format!(
                "Filtered using your saved dining preferences. {}",
                voice_text
            );
            if !needs_reconfirmation.is_empty() {
                voice_text
                    .push_str(" Please confirm if your allergy restrictions are still current.");
            }
        }
    }

    // 5. Serialize structured output
    let structured_val = match serde_json::to_value(&output) {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to serialize SearchMenuOutput: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal serialization error occurred while preparing menu search results.",
            )]);
        }
    };

    let mut tool_result = CallToolResult::structured(structured_val);
    tool_result.content = vec![ContentBlock::text(voice_text)];
    tool_result
}
