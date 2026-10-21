use serde_json::Value;

pub const ALLOWED_TOOLS: &[&str] = &[
    "ping_restaurant",
    "search_menu",
    "get_menu_item",
    "check_availability",
    "create_booking",
    "cancel_booking",
    "get_booking",
    "place_order",
    "get_order",
    "cancel_order",
    "enable_memory",
    "get_profile",
    "remember_preference",
    "forget_preference",
    "get_usual_order",
    "rotate_profile_token",
    "forget_me",
    "negotiate_booking",
    "accept_offer",
    "release_offer",
    "join_waitlist",
    "get_waitlist_status",
    "leave_waitlist",
    "get_deposit_status",
    "refresh_payment_session",
];

pub const DESTRUCTIVE_TOOLS: &[&str] = &[
    "cancel_booking",
    "cancel_order",
    "forget_me",
    "forget_preference",
    "leave_waitlist",
];

pub fn is_tool_allowed(name: &str) -> bool {
    ALLOWED_TOOLS.contains(&name)
}

pub fn is_destructive_tool(name: &str) -> bool {
    DESTRUCTIVE_TOOLS.contains(&name)
}

/// Check if the user's latest text constitutes an explicit confirmation.
pub fn is_explicit_confirmation(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    let confirmation_phrases = [
        "yes",
        "confirm",
        "i confirm",
        "proceed",
        "go ahead",
        "cancel it",
        "yes cancel",
        "please cancel",
        "delete it",
        "yes delete",
        "please delete",
        "sure",
        "ok cancel",
        "leave waitlist",
        "yes please",
    ];

    for phrase in confirmation_phrases {
        if lower == phrase
            || lower.starts_with(&format!("{} ", phrase))
            || lower.ends_with(&format!(" {}", phrase))
            || lower.contains(phrase)
        {
            return true;
        }
    }
    false
}

/// Scrub profile tokens and secret keys from a JSON payload before sending to model.
pub fn scrub_payload_for_model(val: &mut Value) {
    match val {
        Value::Object(map) => {
            map.remove("profile_token");
            map.remove("token");
            map.remove("auth_token");
            for v in map.values_mut() {
                scrub_payload_for_model(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                scrub_payload_for_model(v);
            }
        }
        Value::String(s) if s.contains("tt_prof_") => {
            *s = redact_tokens(s);
        }
        _ => {}
    }
}

/// Redact profile tokens matching `tt_prof_...` from strings.
pub fn redact_tokens(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(pos) = remaining.find("tt_prof_") {
        out.push_str(&remaining[..pos]);
        out.push_str("[REDACTED_PROFILE_TOKEN]");
        let after_prefix = &remaining[pos + 8..];
        let end_idx = after_prefix
            .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .unwrap_or(after_prefix.len());
        remaining = &after_prefix[end_idx..];
    }
    out.push_str(remaining);
    out
}

/// Redact sensitive PII (tokens, emails, phone numbers) for secure application logging.
pub fn redact_for_logging(text: &str) -> String {
    let text = redact_tokens(text);

    // Mask email addresses
    let mut out = String::new();
    for word in text.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        if word.contains('@') && word.contains('.') {
            out.push_str("[REDACTED_EMAIL]");
        } else if word.chars().filter(|c| c.is_ascii_digit()).count() >= 10
            && (word.starts_with('+')
                || word
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '-' || c == ' ' || c == '(' || c == ')'))
        {
            out.push_str("[REDACTED_PHONE]");
        } else {
            out.push_str(word);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_allowed_and_destructive_tools() {
        assert!(is_tool_allowed("check_availability"));
        assert!(is_tool_allowed("cancel_booking"));
        assert!(!is_tool_allowed("unauthorized_tool"));
        assert!(!is_tool_allowed("pay_deposit")); // No pay tool allowed

        assert!(is_destructive_tool("cancel_booking"));
        assert!(is_destructive_tool("cancel_order"));
        assert!(is_destructive_tool("forget_me"));
        assert!(is_destructive_tool("forget_preference"));
        assert!(is_destructive_tool("leave_waitlist"));
        assert!(!is_destructive_tool("check_availability"));
        assert!(!is_destructive_tool("create_booking"));
    }

    #[test]
    fn test_explicit_confirmation() {
        assert!(is_explicit_confirmation("Yes, please cancel it"));
        assert!(is_explicit_confirmation("yes"));
        assert!(is_explicit_confirmation("proceed"));
        assert!(is_explicit_confirmation("Confirm"));
        assert!(!is_explicit_confirmation("No, do not do that"));
        assert!(!is_explicit_confirmation("What time is it?"));
    }

    #[test]
    fn test_scrub_payload_for_model() {
        let mut val = json!({
            "name": "check_availability",
            "profile_token": "tt_prof_abcdef1234567890",
            "nested": {
                "profile_token": "tt_prof_secret",
                "text": "User token is tt_prof_secret_token_123"
            }
        });

        scrub_payload_for_model(&mut val);
        assert!(val.get("profile_token").is_none());
        assert!(val["nested"].get("profile_token").is_none());
        assert!(!val["nested"]["text"].as_str().unwrap().contains("tt_prof_"));
        assert!(val["nested"]["text"]
            .as_str()
            .unwrap()
            .contains("[REDACTED_PROFILE_TOKEN]"));
    }

    #[test]
    fn test_redact_for_logging() {
        let log = "User John Doe with email test@example.com phone +919876543210 and token tt_prof_abcdef";
        let redacted = redact_for_logging(log);
        assert!(!redacted.contains("test@example.com"));
        assert!(!redacted.contains("+919876543210"));
        assert!(!redacted.contains("tt_prof_abcdef"));
        assert!(redacted.contains("[REDACTED_EMAIL]"));
        assert!(redacted.contains("[REDACTED_PROFILE_TOKEN]"));
    }
}
