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
}

/// Helper to produce a tool execution error (isError: true) allowing the LLM to self-correct.
fn tool_validation_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

/// Business logic for executing menu search with hard safety filtering and voice synthesis.
pub async fn handle_search_menu(pool: &SqlitePool, input: SearchMenuInput) -> CallToolResult {
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
    let output = match execute_menu_search(pool, search_spec).await {
        Ok(out) => out,
        Err(err) => {
            tracing::error!("Database query failed during menu search: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal database error occurred while searching the menu. Please try again or ask restaurant staff.",
            )]);
        }
    };

    // 4. Generate voice text (spoken natural English with spoken prices and allergen phrasing)
    let voice_text = generate_voice_text(
        output.total_matches,
        &output.items,
        &output.applied_filters.exclude_allergens,
        &output.suggestions,
    );

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
