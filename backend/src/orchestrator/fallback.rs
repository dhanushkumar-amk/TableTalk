use serde_json::json;
use sqlx::SqlitePool;

use crate::orchestrator::executor::execute_tool;
use crate::orchestrator::types::{ChatMessage, ChatResponse, TokenUsage};

/// Fallback rule-based router when Bedrock is unavailable, throttled, or out of budget.
pub async fn handle_fallback(
    pool: &SqlitePool,
    session_id: &str,
    last_user_message: &str,
    profile_token: Option<&str>,
    restaurant_now: &str,
) -> ChatResponse {
    let lower = last_user_message.to_lowercase();

    // 1. Core Flow: Search Menu
    if lower.contains("menu")
        || lower.contains("food")
        || lower.contains("dish")
        || lower.contains("vegan")
        || lower.contains("starter")
        || lower.contains("dessert")
        || lower.contains("biryani")
        || lower.contains("paneer")
        || lower.contains("tikka")
        || lower.contains("recommend")
    {
        let query = extract_search_query(&lower);
        let args = json!({
            "query": query,
            "limit": 5
        });

        let (text, structured, _) = execute_tool(pool, "search_menu", args, profile_token).await;
        let mut structured_contents = Vec::new();
        if let Some(sc) = structured {
            structured_contents.push(sc);
        }

        return ChatResponse {
            session_id: session_id.to_string(),
            message: ChatMessage {
                role: "assistant".to_string(),
                content: text,
            },
            structured_contents,
            fallback_mode: true,
            token_usage: TokenUsage::default(),
        };
    }

    // 2. Core Flow: Book Table
    if lower.contains("book it")
        || lower.contains("confirm booking")
        || lower.contains("please book")
        || lower.contains("book a table")
    {
        let party_size = extract_party_size(&lower).unwrap_or(2);
        let date = extract_date_or_default(&lower, restaurant_now);
        let guest_name = extract_guest_name(&lower).unwrap_or_else(|| "Dining Guest".to_string());

        let args = json!({
            "date": date,
            "time": "19:30",
            "party_size": party_size,
            "guest_name": guest_name,
            "area": "indoor"
        });

        let (text, structured, _) = execute_tool(pool, "create_booking", args, profile_token).await;
        let mut structured_contents = Vec::new();
        if let Some(sc) = structured {
            structured_contents.push(sc);
        }

        return ChatResponse {
            session_id: session_id.to_string(),
            message: ChatMessage {
                role: "assistant".to_string(),
                content: text,
            },
            structured_contents,
            fallback_mode: true,
            token_usage: TokenUsage::default(),
        };
    }

    // 3. Core Flow: Check Availability
    if lower.contains("table")
        || lower.contains("availab")
        || lower.contains("seat")
        || lower.contains("party")
        || lower.contains("reservation")
    {
        let party_size = extract_party_size(&lower).unwrap_or(2);
        let date = extract_date_or_default(&lower, restaurant_now);

        let args = json!({
            "date": date,
            "time": "19:30",
            "party_size": party_size,
            "area": "any"
        });

        let (text, structured, _) =
            execute_tool(pool, "check_availability", args, profile_token).await;
        let mut structured_contents = Vec::new();
        if let Some(sc) = structured {
            structured_contents.push(sc);
        }

        return ChatResponse {
            session_id: session_id.to_string(),
            message: ChatMessage {
                role: "assistant".to_string(),
                content: text,
            },
            structured_contents,
            fallback_mode: true,
            token_usage: TokenUsage::default(),
        };
    }

    // 4. Core Flow: Order Food
    if lower.contains("order") || lower.contains("takeout") || lower.contains("pickup") {
        let guest_name = extract_guest_name(&lower).unwrap_or_else(|| "Pickup Guest".to_string());
        let args = json!({
            "guest_name": guest_name,
            "items": [
                {
                    "item_name": "Paneer Tikka",
                    "quantity": 1
                }
            ]
        });

        let (text, structured, _) = execute_tool(pool, "place_order", args, profile_token).await;
        let mut structured_contents = Vec::new();
        if let Some(sc) = structured {
            structured_contents.push(sc);
        }

        return ChatResponse {
            session_id: session_id.to_string(),
            message: ChatMessage {
                role: "assistant".to_string(),
                content: text,
            },
            structured_contents,
            fallback_mode: true,
            token_usage: TokenUsage::default(),
        };
    }

    // Default polite voice response
    ChatResponse {
        session_id: session_id.to_string(),
        message: ChatMessage {
            role: "assistant".to_string(),
            content: "Welcome to The Saffron Nebula Bistro. Would you like to check our menu, check table availability, or book a reservation?".to_string(),
        },
        structured_contents: Vec::new(),
        fallback_mode: true,
        token_usage: TokenUsage::default(),
    }
}

fn extract_search_query(text: &str) -> String {
    for word in ["biryani", "paneer", "tikka", "vegan", "starter", "dessert"] {
        if text.contains(word) {
            return word.to_string();
        }
    }
    String::new()
}

fn extract_party_size(text: &str) -> Option<i64> {
    for word in text.split_whitespace() {
        if let Ok(n) = word.parse::<i64>() {
            if (1..=12).contains(&n) {
                return Some(n);
            }
        }
    }
    if text.contains("for two") || text.contains("2 people") || text.contains("2 guests") {
        Some(2)
    } else if text.contains("for four") || text.contains("4 people") || text.contains("4 guests") {
        Some(4)
    } else {
        None
    }
}

fn extract_date_or_default(text: &str, restaurant_now: &str) -> String {
    // If text contains ISO date YYYY-MM-DD
    for word in text.split_whitespace() {
        if word.len() == 10 && word.chars().filter(|c| *c == '-').count() == 2 {
            return word.to_string();
        }
    }
    // Default to tomorrow or date from restaurant_now
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(restaurant_now) {
        let tomorrow = dt.date_naive() + chrono::Duration::days(1);
        tomorrow.format("%Y-%m-%d").to_string()
    } else {
        "2026-10-22".to_string()
    }
}

fn extract_guest_name(text: &str) -> Option<String> {
    if let Some(idx) = text.find("for ") {
        let after = &text[idx + 4..];
        let name_part = after.split_whitespace().next()?;
        if !name_part.chars().any(|c| c.is_ascii_digit()) && name_part.len() > 1 {
            return Some(name_part.to_string());
        }
    }
    None
}
