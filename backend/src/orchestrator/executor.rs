use rmcp::model::{CallToolResult, ContentBlock};
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::orchestrator::safety::is_tool_allowed;
use crate::orchestrator::types::BedrockToolSpec;
use crate::TableTalkServer;

fn tool_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(msg.into())])
}

/// Build compact Bedrock tool specifications from the server's registered tools.
pub fn build_tool_specs(server: &TableTalkServer) -> Vec<BedrockToolSpec> {
    let tools = server.list_registered_tools();
    tools
        .into_iter()
        .filter(|t| is_tool_allowed(&t.name))
        .map(|t| {
            // Requirement 4: Send only compact tool descriptions to save tokens.
            let compact_desc = compact_description(t.description.as_deref().unwrap_or(""));
            let schema_val = serde_json::to_value(&t.input_schema).unwrap_or(json!({
                "type": "object"
            }));

            BedrockToolSpec {
                name: t.name.to_string(),
                description: compact_desc,
                input_schema: schema_val,
            }
        })
        .collect()
}

/// Trims tool description to a compact 1-2 sentence overview.
fn compact_description(desc: &str) -> String {
    let desc = desc.trim();
    if desc.is_empty() {
        return String::new();
    }
    if let Some(period_idx) = desc.find(". ") {
        if let Some(second_idx) = desc[period_idx + 2..].find(". ") {
            desc[..period_idx + 2 + second_idx + 1].to_string()
        } else {
            desc[..period_idx + 1].to_string()
        }
    } else if desc.len() > 160 {
        format!("{}...", &desc[..157])
    } else {
        desc.to_string()
    }
}

/// Execute an MCP tool in-process using the database connection pool.
/// Injects server-held profile_token into tool arguments if available.
pub async fn execute_tool(
    pool: &SqlitePool,
    name: &str,
    mut args: Value,
    profile_token: Option<&str>,
) -> (String, Option<Value>, bool) {
    if !is_tool_allowed(name) {
        return (
            format!("Tool '{}' is not recognized or not permitted.", name),
            None,
            true,
        );
    }

    // Server-side profile_token injection
    if let Some(token) = profile_token {
        if args.is_object() && args.get("profile_token").is_none() {
            args["profile_token"] = json!(token);
        }
    }

    let result = dispatch_tool(pool, name, args).await;
    extract_result_parts(result)
}

async fn dispatch_tool(pool: &SqlitePool, name: &str, args: Value) -> CallToolResult {
    match name {
        "ping_restaurant" => {
            let input: crate::tools::ping::PingInput = match serde_json::from_value(args) {
                Ok(inp) => inp,
                Err(e) => return tool_error(format!("Invalid arguments for ping_restaurant: {e}")),
            };
            let output = crate::tools::ping::handle_ping(input);
            CallToolResult::success(vec![ContentBlock::text(
                serde_json::to_string(&output).unwrap_or_default(),
            )])
        }
        "search_menu" => {
            let input: crate::tools::search_menu::SearchMenuInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => return tool_error(format!("Invalid arguments for search_menu: {e}")),
                };
            crate::tools::search_menu::handle_search_menu(pool, input).await
        }
        "check_availability" => {
            let input: crate::tools::check_availability::CheckAvailabilityInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for check_availability: {e}"))
                    }
                };
            crate::tools::check_availability::handle_check_availability(pool, input).await
        }
        "create_booking" => {
            let input: crate::tools::create_booking::CreateBookingInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for create_booking: {e}"))
                    }
                };
            crate::tools::create_booking::handle_create_booking(pool, input).await
        }
        "cancel_booking" => {
            let input: crate::tools::cancel_booking::CancelBookingInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for cancel_booking: {e}"))
                    }
                };
            crate::tools::cancel_booking::handle_cancel_booking(pool, input).await
        }
        "get_booking" => {
            let input: crate::tools::get_booking::GetBookingInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => return tool_error(format!("Invalid arguments for get_booking: {e}")),
                };
            crate::tools::get_booking::handle_get_booking(pool, input).await
        }
        "place_order" => {
            let input: crate::tools::place_order::PlaceOrderInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => return tool_error(format!("Invalid arguments for place_order: {e}")),
                };
            crate::tools::place_order::handle_place_order(pool, input).await
        }
        "get_order" => {
            let input: crate::tools::get_order::GetOrderInput = match serde_json::from_value(args) {
                Ok(inp) => inp,
                Err(e) => return tool_error(format!("Invalid arguments for get_order: {e}")),
            };
            crate::tools::get_order::handle_get_order(pool, input).await
        }
        "cancel_order" => {
            let input: crate::tools::cancel_order::CancelOrderInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for cancel_order: {e}"))
                    }
                };
            crate::tools::cancel_order::handle_cancel_order(pool, input).await
        }
        "enable_memory" => {
            let input: crate::tools::enable_memory::EnableMemoryInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for enable_memory: {e}"))
                    }
                };
            crate::tools::enable_memory::handle_enable_memory(pool, input).await
        }
        "get_profile" => {
            let input: crate::tools::get_profile::GetProfileInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => return tool_error(format!("Invalid arguments for get_profile: {e}")),
                };
            crate::tools::get_profile::handle_get_profile(pool, input).await
        }
        "remember_preference" => {
            let input: crate::tools::remember_preference::RememberPreferenceInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!(
                            "Invalid arguments for remember_preference: {e}"
                        ))
                    }
                };
            crate::tools::remember_preference::handle_remember_preference(pool, input).await
        }
        "forget_preference" => {
            let input: crate::tools::forget_preference::ForgetPreferenceInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for forget_preference: {e}"))
                    }
                };
            crate::tools::forget_preference::handle_forget_preference(pool, input).await
        }
        "get_usual_order" => {
            let input: crate::tools::get_usual_order::GetUsualOrderInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for get_usual_order: {e}"))
                    }
                };
            crate::tools::get_usual_order::handle_get_usual_order(pool, input).await
        }
        "rotate_profile_token" => {
            let input: crate::tools::rotate_profile_token::RotateProfileTokenInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!(
                            "Invalid arguments for rotate_profile_token: {e}"
                        ))
                    }
                };
            crate::tools::rotate_profile_token::handle_rotate_profile_token(pool, input).await
        }
        "forget_me" => {
            let input: crate::tools::forget_me::ForgetMeInput = match serde_json::from_value(args) {
                Ok(inp) => inp,
                Err(e) => return tool_error(format!("Invalid arguments for forget_me: {e}")),
            };
            crate::tools::forget_me::handle_forget_me(pool, input).await
        }
        "negotiate_booking" => {
            let input: crate::tools::negotiate_booking::NegotiateBookingInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for negotiate_booking: {e}"))
                    }
                };
            crate::tools::negotiate_booking::handle_negotiate_booking(pool, input).await
        }
        "accept_offer" => {
            let input: crate::tools::accept_offer::AcceptOfferInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for accept_offer: {e}"))
                    }
                };
            crate::tools::accept_offer::handle_accept_offer(pool, input).await
        }
        "release_offer" => {
            let input: crate::tools::release_offer::ReleaseOfferInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for release_offer: {e}"))
                    }
                };
            crate::tools::release_offer::handle_release_offer(pool, input).await
        }
        "join_waitlist" => {
            let input: crate::tools::join_waitlist::JoinWaitlistInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for join_waitlist: {e}"))
                    }
                };
            crate::tools::join_waitlist::handle_join_waitlist(pool, input).await
        }
        "get_waitlist_status" => {
            let input: crate::tools::get_waitlist_status::GetWaitlistStatusInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!(
                            "Invalid arguments for get_waitlist_status: {e}"
                        ))
                    }
                };
            crate::tools::get_waitlist_status::handle_get_waitlist_status(pool, input).await
        }
        "leave_waitlist" => {
            let input: crate::tools::leave_waitlist::LeaveWaitlistInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for leave_waitlist: {e}"))
                    }
                };
            crate::tools::leave_waitlist::handle_leave_waitlist(pool, input).await
        }
        "get_deposit_status" => {
            let input: crate::tools::get_deposit_status::GetDepositStatusInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!("Invalid arguments for get_deposit_status: {e}"))
                    }
                };
            crate::tools::get_deposit_status::handle_get_deposit_status(pool, input).await
        }
        "refresh_payment_session" => {
            let input: crate::tools::refresh_payment_session::RefreshPaymentSessionInput =
                match serde_json::from_value(args) {
                    Ok(inp) => inp,
                    Err(e) => {
                        return tool_error(format!(
                            "Invalid arguments for refresh_payment_session: {e}"
                        ))
                    }
                };
            crate::tools::refresh_payment_session::handle_refresh_payment_session(pool, input).await
        }
        _ => tool_error(format!("Tool '{name}' is not supported.")),
    }
}

fn extract_result_parts(result: CallToolResult) -> (String, Option<Value>, bool) {
    let is_error = result.is_error.unwrap_or(false);
    let mut text_parts = Vec::new();
    for block in &result.content {
        if let ContentBlock::Text(t) = block {
            text_parts.push(t.text.clone());
        }
    }
    let text = if text_parts.is_empty() {
        if is_error {
            "Tool execution failed.".to_string()
        } else {
            "Tool execution completed.".to_string()
        }
    } else {
        text_parts.join("\n")
    };

    let structured_content = result.structured_content;
    (text, structured_content, is_error)
}
