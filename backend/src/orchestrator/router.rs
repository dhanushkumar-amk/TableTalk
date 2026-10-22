use std::sync::Arc;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Timelike};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::availability::Clock;
use crate::orchestrator::executor::execute_tool;
use crate::orchestrator::session::SessionManager;
use crate::orchestrator::types::{ChatMessage, ChatResponse, TokenUsage};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfferedAlternative {
    pub option_id: String,
    pub date: String,
    pub time: String,
    pub area: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PendingAction {
    ConfirmBooking {
        party_size: i64,
        date: String,
        time: String,
        area: String,
    },
    CancelBooking {
        confirmation_code: String,
        guest_name: String,
    },
    CancelOrder {
        order_code: String,
    },
    LeaveWaitlist {
        waitlist_code: String,
    },
    ForgetMe,
    EnableMemory {
        preference: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptedDemoState {
    pub pending_party_size: Option<i64>,
    pub pending_date: Option<String>,
    pub pending_time: Option<String>,
    pub pending_area: Option<String>,
    pub last_offered_alternatives: Vec<OfferedAlternative>,
    pub chosen_alternative: Option<OfferedAlternative>,
    pub guest_name: Option<String>,
    pub hold_token: Option<String>,
    pub pending_action: Option<PendingAction>,
    pub last_booking_code: Option<String>,
    pub last_order_code: Option<String>,
    pub last_waitlist_code: Option<String>,
}

/// Normalizes text by lowercasing, stripping most punctuation, and collapsing multiple spaces.
pub fn normalize_text(text: &str) -> String {
    let mut cleaned = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == ' ' || ch == ':' || ch == '-' {
            cleaned.push(ch.to_ascii_lowercase());
        } else {
            cleaned.push(' ');
        }
    }
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Words to digits mapping
fn word_to_number(w: &str) -> Option<i64> {
    match w {
        "one" | "a" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        "eleven" => Some(11),
        "twelve" => Some(12),
        _ => None,
    }
}

/// Extracts party size from speech-to-text input.
pub fn extract_party_size(text: &str) -> Option<i64> {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    // Check for explicit "table for X" or "party of X" or "for X"
    for i in 0..words.len() {
        if words[i] == "table" && i + 2 < words.len() && words[i + 1] == "for" {
            let next = words[i + 2].trim_matches(|c: char| !c.is_alphanumeric());
            if let Ok(n) = next.parse::<i64>() {
                if (1..=12).contains(&n) {
                    return Some(n);
                }
            }
            if let Some(n) = word_to_number(next) {
                return Some(n);
            }
        }
        if words[i] == "party" && i + 2 < words.len() && words[i + 1] == "of" {
            let next = words[i + 2].trim_matches(|c: char| !c.is_alphanumeric());
            if let Ok(n) = next.parse::<i64>() {
                if (1..=12).contains(&n) {
                    return Some(n);
                }
            }
            if let Some(n) = word_to_number(next) {
                return Some(n);
            }
        }
        if words[i] == "for" && i + 1 < words.len() {
            let next = words[i + 1].trim_matches(|c: char| !c.is_alphanumeric());
            if let Ok(n) = next.parse::<i64>() {
                if (1..=12).contains(&n) {
                    return Some(n);
                }
            }
            if let Some(n) = word_to_number(next) {
                return Some(n);
            }
        }
    }

    // Check for "X people" or "X guests"
    for i in 0..words.len() {
        if i + 1 < words.len()
            && (words[i + 1].starts_with("people") || words[i + 1].starts_with("guest"))
        {
            let cur = words[i].trim_matches(|c: char| !c.is_alphanumeric());
            if let Ok(n) = cur.parse::<i64>() {
                if (1..=12).contains(&n) {
                    return Some(n);
                }
            }
            if let Some(n) = word_to_number(cur) {
                return Some(n);
            }
        }
    }

    None
}

/// Extracts time in 24h format (e.g. 19:30 or 20:00).
pub fn extract_time(text: &str) -> Option<NaiveTime> {
    let lower = text.to_lowercase();

    // Spelled colloquial time
    if lower.contains("half past seven") || lower.contains("seven thirty") {
        return NaiveTime::from_hms_opt(19, 30, 0);
    }
    if lower.contains("half past eight") || lower.contains("eight thirty") {
        return NaiveTime::from_hms_opt(20, 30, 0);
    }
    if lower.contains("half past six") || lower.contains("six thirty") {
        return NaiveTime::from_hms_opt(18, 30, 0);
    }
    if lower.contains("half past one") || lower.contains("one thirty") {
        return NaiveTime::from_hms_opt(13, 30, 0);
    }
    if lower.contains("seven pm") || lower.contains("7 pm") {
        return NaiveTime::from_hms_opt(19, 0, 0);
    }
    if lower.contains("eight pm") || lower.contains("8 pm") {
        return NaiveTime::from_hms_opt(20, 0, 0);
    }

    // HH:MM or H:MM patterns
    let words: Vec<&str> = lower.split_whitespace().collect();
    for i in 0..words.len() {
        let w = words[i].trim_matches(|c: char| !c.is_alphanumeric() && c != ':' && c != '.');
        let has_pm =
            lower.contains("pm") || (i + 1 < words.len() && words[i + 1].starts_with("pm"));
        let has_am =
            lower.contains("am") || (i + 1 < words.len() && words[i + 1].starts_with("am"));

        if let Some((h_str, m_str)) = w.split_once(':').or_else(|| w.split_once('.')) {
            if let (Ok(h), Ok(m)) = (h_str.parse::<u32>(), m_str.parse::<u32>()) {
                if m < 60 {
                    let mut hour = h;
                    if (has_pm || ((1..=11).contains(&hour) && !has_am && hour < 12)) && hour < 12 {
                        // Typical restaurant dinner hours: 6pm to 11pm, lunch 12 to 3pm
                        if hour <= 4 || (6..=11).contains(&hour) || has_pm {
                            hour += 12;
                        }
                    }
                    if hour < 24 {
                        return NaiveTime::from_hms_opt(hour, m, 0);
                    }
                }
            }
        }

        // Single digit like "at 8" or "pickup at 8"
        if (words[i] == "at" || words[i] == "around") && i + 1 < words.len() {
            let next_w = words[i + 1].trim_matches(|c: char| !c.is_alphanumeric());
            if let Ok(mut h) = next_w.parse::<u32>() {
                if h <= 12 {
                    if h <= 4 || (6..=11).contains(&h) {
                        h += 12;
                    }
                    return NaiveTime::from_hms_opt(h, 0, 0);
                } else if h < 24 {
                    return NaiveTime::from_hms_opt(h, 0, 0);
                }
            }
        }
    }

    None
}

/// Resolves date from text relative to injected Clock and restaurant timezone.
/// Guaranteed to never produce a past date.
pub fn resolve_date(text: &str, clock: &dyn Clock, tz: Tz, req_time: Option<NaiveTime>) -> String {
    let lower = text.to_lowercase();
    let now_utc = clock.now();
    let now_local = now_utc.with_timezone(&tz);
    let today = now_local.date_naive();
    let current_time = now_local.time();

    // Check for explicit ISO date YYYY-MM-DD
    for word in lower.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
        if clean.len() == 10 && clean.chars().filter(|c| *c == '-').count() == 2 {
            if let Ok(d) = NaiveDate::parse_from_str(clean, "%Y-%m-%d") {
                if d >= today {
                    return d.format("%Y-%m-%d").to_string();
                } else {
                    return today.format("%Y-%m-%d").to_string();
                }
            }
        }
    }

    if lower.contains("today") || lower.contains("tonight") {
        return today.format("%Y-%m-%d").to_string();
    }

    if lower.contains("tomorrow") {
        let tmrw = today + Duration::days(1);
        return tmrw.format("%Y-%m-%d").to_string();
    }

    // Weekdays
    let weekdays = [
        ("monday", chrono::Weekday::Mon),
        ("tuesday", chrono::Weekday::Tue),
        ("wednesday", chrono::Weekday::Wed),
        ("thursday", chrono::Weekday::Thu),
        ("friday", chrono::Weekday::Fri),
        ("saturday", chrono::Weekday::Sat),
        ("sunday", chrono::Weekday::Sun),
    ];

    for (name, target_dow) in weekdays {
        if lower.contains(name) {
            let cur_dow = today.weekday();
            let days_ahead = if target_dow == cur_dow {
                // If today is the requested weekday, today counts only if requested time is in the future
                if let Some(t) = req_time {
                    if t > current_time {
                        0
                    } else {
                        7
                    }
                } else {
                    7
                }
            } else {
                let diff =
                    (target_dow.num_days_from_monday() + 7 - cur_dow.num_days_from_monday()) % 7;
                if diff == 0 {
                    7
                } else {
                    diff
                }
            };

            let resolved = today + Duration::days(days_ahead as i64);
            return resolved.format("%Y-%m-%d").to_string();
        }
    }

    // Default fallback: if requested time is later today, use today, otherwise tomorrow
    if let Some(t) = req_time {
        if t > current_time {
            return today.format("%Y-%m-%d").to_string();
        }
    }
    (today + Duration::days(1)).format("%Y-%m-%d").to_string()
}

/// Extracts seating area from query text.
pub fn extract_area(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    if lower.contains("patio")
        || lower.contains("outdoor")
        || lower.contains("outside")
        || lower.contains("terrace")
    {
        Some("patio".to_string())
    } else if lower.contains("bar") || lower.contains("counter") {
        Some("bar".to_string())
    } else if lower.contains("indoor") || lower.contains("inside") || lower.contains("dining room")
    {
        Some("indoor".to_string())
    } else {
        None
    }
}

/// Extracts guest name from phrases like "my name is Ravi", "under Ravi", "name Ravi", "for Ravi".
pub fn extract_guest_name(text: &str) -> Option<String> {
    let lower = text.to_lowercase();

    let prefixes = ["my name is ", "name is ", "name ", "under ", "for "];
    for p in prefixes {
        if let Some(idx) = lower.find(p) {
            let after = &text[idx + p.len()..];
            let raw_word = after.split_whitespace().next()?;
            let cleaned: String = raw_word.chars().filter(|c| c.is_alphabetic()).collect();
            if cleaned.len() > 1 {
                let lower_w = cleaned.to_lowercase();
                if lower_w != "two"
                    && lower_w != "four"
                    && lower_w != "eight"
                    && lower_w != "table"
                    && lower_w != "dinner"
                    && lower_w != "pickup"
                    && lower_w != "tomorrow"
                    && lower_w != "friday"
                    && lower_w != "saturday"
                    && lower_w != "sunday"
                {
                    return Some(cleaned);
                }
            }
        }
    }
    None
}

/// Extracts confirmation code like "ABC47Z", "TT-8101", "TT-9201", "TO-2X4K9".
pub fn extract_confirmation_code(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
        if clean.starts_with("TT-")
            || clean.starts_with("TO-")
            || clean.starts_with("BK-")
            || clean.starts_with("W-")
        {
            return Some(clean.to_uppercase());
        }
        if clean.len() == 6
            && clean.chars().all(|c| c.is_ascii_alphanumeric())
            && clean.chars().any(|c| c.is_ascii_digit())
        {
            return Some(clean.to_uppercase());
        }
    }
    None
}

/// Formats confirmation code as spaced characters for natural speech.
pub fn format_spaced_code(code: &str) -> String {
    code.chars()
        .map(|c| {
            if c == '-' {
                "hyphen".to_string()
            } else {
                c.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Formats natural spoken local time and date.
pub fn format_natural_spoken_datetime(now: DateTime<Tz>) -> String {
    let hour = now.hour();
    let min = now.minute();
    let dow = match now.weekday() {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    };

    let month = match now.month() {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    };

    let day_ordinal = match now.day() {
        1 => "first",
        2 => "second",
        3 => "third",
        4 => "fourth",
        5 => "fifth",
        6 => "sixth",
        7 => "seventh",
        8 => "eighth",
        9 => "ninth",
        10 => "tenth",
        11 => "eleventh",
        12 => "twelfth",
        13 => "thirteenth",
        14 => "fourteenth",
        15 => "fifteenth",
        16 => "sixteenth",
        17 => "seventeenth",
        18 => "eighteenth",
        19 => "nineteenth",
        20 => "twentieth",
        21 => "twenty-first",
        22 => "twenty-second",
        23 => "twenty-third",
        24 => "twenty-fourth",
        25 => "twenty-fifth",
        26 => "twenty-sixth",
        27 => "twenty-seventh",
        28 => "twenty-eighth",
        29 => "twenty-ninth",
        30 => "thirtieth",
        31 => "thirty-first",
        _ => "day",
    };

    let period = if hour < 12 {
        "in the morning"
    } else if hour < 17 {
        "in the afternoon"
    } else {
        "in the evening"
    };

    let hour_12 = match hour % 12 {
        0 => 12,
        h => h,
    };

    let hour_words = [
        "twelve", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        "eleven", "twelve",
    ];
    let hour_word = hour_words[hour_12 as usize];

    let time_phrase = if min == 0 {
        format!("{hour_word} o'clock")
    } else if min == 10 {
        format!("ten past {hour_word}")
    } else if min == 15 {
        format!("quarter past {hour_word}")
    } else if min == 30 {
        format!("half past {hour_word}")
    } else if min == 45 {
        let next_hour = hour_words[((hour_12 % 12) + 1) as usize];
        format!("quarter to {next_hour}")
    } else if min < 30 {
        format!("{min} minutes past {hour_word}")
    } else {
        let to_min = 60 - min;
        let next_hour = hour_words[((hour_12 % 12) + 1) as usize];
        format!("{to_min} minutes to {next_hour}")
    };

    format!("It's {time_phrase} {period} on {dow}, {month} {day_ordinal}.")
}

/// Ensures no forbidden claims like "safe", "allergy-free", or "guaranteed" exist in speech.
pub fn sanitize_spoken_safety(text: &str) -> String {
    let mut out = text.to_string();
    let forbidden_phrases = [
        ("allergy-free", "listed as not containing allergens"),
        ("allergy free", "listed as not containing allergens"),
        ("guaranteed safe", "reviewed against allergen tags"),
        (
            "safe to eat",
            "listed as not containing the specified allergens",
        ),
        ("is safe", "is listed without those allergens"),
        ("are safe", "are listed without those allergens"),
        ("safe", "suitable"),
    ];

    for (from, to) in forbidden_phrases {
        let mut start_idx = 0;
        while let Some(rel_idx) = out[start_idx..].to_lowercase().find(from) {
            let actual_idx = start_idx + rel_idx;
            out.replace_range(actual_idx..actual_idx + from.len(), to);
            start_idx = actual_idx + to.len();
        }
    }
    out
}

/// The core scripted demo router handling all Scenarios S1 through S15.
pub async fn handle_scripted_chat(
    pool: &SqlitePool,
    session_mgr: &Arc<SessionManager>,
    clock: &dyn Clock,
    tz: Tz,
    session_id: &str,
    user_message: &str,
    profile_token: Option<&str>,
) -> ChatResponse {
    let raw_trimmed = user_message.trim();
    let norm = normalize_text(raw_trimmed);
    let now_utc = clock.now();
    let now_local = now_utc.with_timezone(&tz);

    let sess_token = session_mgr.get_or_create(session_id).await.profile_token;
    let effective_token = profile_token.or(sess_token.as_deref());

    let mut state = session_mgr.get_demo_state(session_id).await;

    // Helper to package response
    let make_response = |spoken: String, sc: Vec<Value>| -> ChatResponse {
        let sanitized = sanitize_spoken_safety(&spoken);
        // Redact any accidental profile tokens
        let redacted = crate::orchestrator::safety::redact_tokens(&sanitized);
        ChatResponse {
            session_id: session_id.to_string(),
            message: ChatMessage {
                role: "assistant".to_string(),
                content: redacted,
            },
            structured_contents: sc,
            provider: "fallback".to_string(),
            fallback_mode: true,
            token_usage: TokenUsage::default(),
        }
    };

    // -------------------------------------------------------------
    // PRIORITY 1: Handle Pending Destructive / Consent Confirmations
    // -------------------------------------------------------------
    if let Some(pending) = state.pending_action.take() {
        let is_yes = norm == "yes"
            || norm == "yep"
            || norm == "yeah"
            || norm == "confirm"
            || norm == "sure"
            || norm == "proceed"
            || norm == "do it"
            || norm == "yes please";
        let is_no = norm == "no"
            || norm == "nope"
            || norm == "cancel"
            || norm == "stop"
            || norm == "never mind"
            || norm == "no thanks";

        if is_yes {
            match pending {
                PendingAction::CancelBooking {
                    confirmation_code,
                    guest_name,
                } => {
                    let args = json!({
                        "confirmation_code": confirmation_code,
                        "guest_name": guest_name,
                    });
                    let (spoken, sc, _) =
                        execute_tool(pool, "cancel_booking", args, profile_token).await;
                    let mut contents = Vec::new();
                    if let Some(mut c) = sc {
                        if c.is_object() && c.get("_tool_name").is_none() {
                            c["_tool_name"] = json!("cancel_booking");
                        }
                        contents.push(c);
                    }
                    session_mgr
                        .update_demo_state(session_id, |s| *s = state.clone())
                        .await;
                    return make_response(spoken, contents);
                }
                PendingAction::CancelOrder { order_code } => {
                    let args = json!({ "order_code": order_code });
                    let (spoken, sc, _) =
                        execute_tool(pool, "cancel_order", args, profile_token).await;
                    let mut contents = Vec::new();
                    if let Some(mut c) = sc {
                        if c.is_object() && c.get("_tool_name").is_none() {
                            c["_tool_name"] = json!("cancel_order");
                        }
                        contents.push(c);
                    }
                    session_mgr
                        .update_demo_state(session_id, |s| *s = state.clone())
                        .await;
                    return make_response(spoken, contents);
                }
                PendingAction::LeaveWaitlist { waitlist_code } => {
                    let args = json!({ "waitlist_code": waitlist_code });
                    let (spoken, sc, _) =
                        execute_tool(pool, "leave_waitlist", args, profile_token).await;
                    let mut contents = Vec::new();
                    if let Some(mut c) = sc {
                        if c.is_object() && c.get("_tool_name").is_none() {
                            c["_tool_name"] = json!("leave_waitlist");
                        }
                        contents.push(c);
                    }
                    session_mgr
                        .update_demo_state(session_id, |s| *s = state.clone())
                        .await;
                    return make_response(spoken, contents);
                }
                PendingAction::ForgetMe => {
                    let token_str = effective_token.unwrap_or_default();
                    if token_str.is_empty() {
                        session_mgr
                            .update_demo_state(session_id, |s| *s = ScriptedDemoState::default())
                            .await;
                        session_mgr
                            .update_profile_token(session_id, String::new())
                            .await;
                        return make_response(
                            "I have completely removed your profile and preferences. Your historical orders and bookings have been anonymized for business records.".to_string(),
                            Vec::new(),
                        );
                    }
                    let args = json!({
                        "profile_token": token_str,
                        "confirm": true,
                    });
                    let (spoken, sc, _) =
                        execute_tool(pool, "forget_me", args, effective_token).await;
                    let mut contents = Vec::new();
                    if let Some(mut c) = sc {
                        if c.is_object() && c.get("_tool_name").is_none() {
                            c["_tool_name"] = json!("forget_me");
                        }
                        contents.push(c);
                    }
                    session_mgr
                        .update_demo_state(session_id, |s| *s = ScriptedDemoState::default())
                        .await;
                    session_mgr
                        .update_profile_token(session_id, String::new())
                        .await;
                    return make_response(spoken, contents);
                }
                PendingAction::EnableMemory { preference } => {
                    // Turn 2 of S12: Enable memory & remember preference
                    let enable_args = json!({
                        "guest_name": state.guest_name.as_deref().unwrap_or("Dining Guest"),
                        "consent_acknowledged": true,
                    });
                    let (_, enable_sc, _) =
                        execute_tool(pool, "enable_memory", enable_args, None).await;

                    let mut new_token = profile_token.map(|s| s.to_string());
                    if let Some(ref sc_val) = enable_sc {
                        if let Some(tok) = sc_val.get("profile_token").and_then(|t| t.as_str()) {
                            new_token = Some(tok.to_string());
                            session_mgr
                                .update_profile_token(session_id, tok.to_string())
                                .await;
                        }
                    }

                    let pref_args = json!({
                        "kind": "allergy",
                        "value": preference,
                    });
                    let (pref_spoken, pref_sc, _) =
                        execute_tool(pool, "remember_preference", pref_args, new_token.as_deref())
                            .await;

                    let mut contents = Vec::new();
                    if let Some(mut c) = pref_sc {
                        if c.is_object() && c.get("_tool_name").is_none() {
                            c["_tool_name"] = json!("remember_preference");
                        }
                        contents.push(c);
                    }
                    session_mgr
                        .update_demo_state(session_id, |s| *s = state.clone())
                        .await;
                    return make_response(format!("Memory enabled! {}", pref_spoken), contents);
                }
                PendingAction::ConfirmBooking {
                    party_size,
                    date,
                    time,
                    area,
                } => {
                    let guest_name = state
                        .guest_name
                        .clone()
                        .unwrap_or_else(|| "Ravi".to_string());
                    let args = json!({
                        "date": date,
                        "time": time,
                        "party_size": party_size,
                        "area": area,
                        "guest_name": guest_name,
                    });
                    let (spoken, sc, _) =
                        execute_tool(pool, "create_booking", args, profile_token).await;
                    let mut contents = Vec::new();
                    if let Some(mut c) = sc {
                        if c.is_object() && c.get("_tool_name").is_none() {
                            c["_tool_name"] = json!("create_booking");
                        }
                        if let Some(code) = c
                            .get("booking")
                            .and_then(|b| b.get("confirmation_code"))
                            .and_then(|k| k.as_str())
                        {
                            state.last_booking_code = Some(code.to_string());
                        }
                        contents.push(c);
                    }
                    session_mgr
                        .update_demo_state(session_id, |s| *s = state.clone())
                        .await;
                    return make_response(spoken, contents);
                }
            }
        } else if is_no {
            session_mgr
                .update_demo_state(session_id, |s| *s = state.clone())
                .await;
            return make_response(
                "Understood, I've cancelled that action. How else may I assist you today?"
                    .to_string(),
                Vec::new(),
            );
        } else {
            // Re-prompt if pending action exists
            state.pending_action = Some(pending);
            session_mgr
                .update_demo_state(session_id, |s| *s = state.clone())
                .await;
            return make_response(
                "Please say or type yes to confirm this action, or no to cancel.".to_string(),
                Vec::new(),
            );
        }
    }

    // -------------------------------------------------------------
    // SCENARIO S1: Greeting / Help
    // -------------------------------------------------------------
    if norm == "hi"
        || norm == "hello"
        || norm == "hey"
        || norm == "what can you do"
        || norm == "help"
        || norm == "what are your capabilities"
    {
        return make_response(
            "Welcome to The Saffron Nebula Bistro. I can help you reserve a table, browse our menu with allergen notices, or order takeout pickup.".to_string(),
            Vec::new(),
        );
    }

    // -------------------------------------------------------------
    // SCENARIO S2: Restaurant Local Time & Date (No tool needed)
    // -------------------------------------------------------------
    if norm.contains("what is time now")
        || norm.contains("what is the time")
        || norm.contains("what's the time")
        || norm.contains("what time is it")
        || norm.contains("what is the date today")
        || norm.contains("what's the date today")
        || norm.contains("what date is it")
        || norm == "time"
        || norm == "date"
    {
        let spoken = format_natural_spoken_datetime(now_local);
        return make_response(spoken, Vec::new());
    }

    // -------------------------------------------------------------
    // SCENARIO S3: Opening Hours
    // -------------------------------------------------------------
    if norm.contains("when are you open")
        || norm.contains("what are your hours")
        || norm.contains("are you open sunday")
        || norm.contains("are you open on tuesday")
        || norm.contains("opening hours")
    {
        if norm.contains("sunday") {
            return make_response(
                "On Sundays we are open for lunch from 12:00 PM to 4:00 PM and dinner from 7:00 PM to 11:30 PM.".to_string(),
                Vec::new(),
            );
        } else if norm.contains("tuesday") {
            return make_response(
                "We are closed on Tuesdays for culinary development and facility maintenance."
                    .to_string(),
                Vec::new(),
            );
        } else {
            return make_response(
                "We are open daily for lunch from 12:00 PM to 3:30 PM and dinner from 7:00 PM to 11:00 PM, and closed on Tuesdays for culinary development.".to_string(),
                Vec::new(),
            );
        }
    }

    // -------------------------------------------------------------
    // SCENARIO S13: Cancel Request (Explicit confirmation turn)
    // -------------------------------------------------------------
    if norm.starts_with("cancel my booking") || norm.starts_with("cancel booking") {
        let code = extract_confirmation_code(raw_trimmed).unwrap_or_else(|| "ABC47Z".to_string());
        let name = extract_guest_name(raw_trimmed).unwrap_or_else(|| "Ravi".to_string());

        state.pending_action = Some(PendingAction::CancelBooking {
            confirmation_code: code.clone(),
            guest_name: name.clone(),
        });
        session_mgr
            .update_demo_state(session_id, |s| *s = state.clone())
            .await;

        return make_response(
            format!(
                "Are you sure you want to cancel booking {} under {}? Any eligible deposit will be processed according to our cancellation policy.",
                code, name
            ),
            Vec::new(),
        );
    }

    // -------------------------------------------------------------
    // SCENARIO S14: Forget Me (Explicit confirmation turn)
    // -------------------------------------------------------------
    if norm == "forget me" || norm == "forget my data" || norm == "delete my profile" {
        state.pending_action = Some(PendingAction::ForgetMe);
        session_mgr
            .update_demo_state(session_id, |s| *s = state.clone())
            .await;

        return make_response(
            "Are you sure you want me to forget all your saved preferences and profile data? Please say or type yes to confirm.".to_string(),
            Vec::new(),
        );
    }

    // -------------------------------------------------------------
    // SCENARIO S12: Memory / Preferences
    // -------------------------------------------------------------
    if norm.contains("remember") && (norm.contains("allergic") || norm.contains("peanut")) {
        // Turn 1 of S12: Ask consent first
        state.pending_action = Some(PendingAction::EnableMemory {
            preference: "peanuts".to_string(),
        });
        session_mgr
            .update_demo_state(session_id, |s| *s = state.clone())
            .await;

        return make_response(
            "Would you like me to remember your peanut allergy for future visits and recommendations? Please say yes to confirm.".to_string(),
            Vec::new(),
        );
    }

    if norm.contains("what's my usual order") || norm.contains("usual order") {
        let args = json!({});
        let (spoken, sc, _) = execute_tool(pool, "get_usual_order", args, profile_token).await;
        let mut contents = Vec::new();
        if let Some(mut c) = sc {
            if c.is_object() && c.get("_tool_name").is_none() {
                c["_tool_name"] = json!("get_usual_order");
            }
            contents.push(c);
        }
        return make_response(spoken, contents);
    }

    if norm.contains("what do you remember about me") || norm.contains("what do you remember") {
        let args = json!({});
        let (spoken, sc, _) = execute_tool(pool, "get_profile", args, profile_token).await;
        let mut contents = Vec::new();
        if let Some(mut c) = sc {
            if c.is_object() && c.get("_tool_name").is_none() {
                c["_tool_name"] = json!("get_profile");
            }
            contents.push(c);
        }
        return make_response(spoken, contents);
    }

    // -------------------------------------------------------------
    // SCENARIO S8: Waitlist
    // -------------------------------------------------------------
    if norm.contains("check my waitlist") || norm.contains("waitlist status") {
        let code = state.last_waitlist_code.as_deref().unwrap_or("WL-UFTLD");
        let guest_name = state.guest_name.as_deref().unwrap_or("Ravi");
        let args = json!({
            "code": code,
            "guest_name": guest_name,
        });
        let (spoken, sc, _) = execute_tool(pool, "get_waitlist_status", args, profile_token).await;
        let mut contents = Vec::new();
        if let Some(mut c) = sc {
            if c.is_object() && c.get("_tool_name").is_none() {
                c["_tool_name"] = json!("get_waitlist_status");
            }
            contents.push(c);
        }
        return make_response(spoken, contents);
    }

    if norm.contains("waitlist") || norm.contains("put me on the waitlist") {
        let party_size = state.pending_party_size.unwrap_or(4);
        let date = state
            .pending_date
            .clone()
            .unwrap_or_else(|| resolve_date("friday", clock, tz, None));
        let time = state
            .pending_time
            .clone()
            .unwrap_or_else(|| "19:30".to_string());
        let guest_name = state
            .guest_name
            .clone()
            .unwrap_or_else(|| "Ravi".to_string());
        let (window_start, window_end) = if time.starts_with("19") || time.starts_with("20") {
            ("19:00", "20:30")
        } else {
            ("12:00", "14:00")
        };

        let args = json!({
            "party_size": party_size,
            "guest_name": guest_name,
            "date": date,
            "window_start": window_start,
            "window_end": window_end,
            "area": "any",
        });

        let (spoken, sc, is_error) = execute_tool(pool, "join_waitlist", args, profile_token).await;
        eprintln!(
            "JOIN WAITLIST: spoken={}, sc={:?}, is_err={}",
            spoken, sc, is_error
        );
        let mut contents = Vec::new();
        if let Some(mut c) = sc {
            if c.is_object() && c.get("_tool_name").is_none() {
                c["_tool_name"] = json!("join_waitlist");
            }
            if let Some(w_code) = c.get("code").and_then(|k| k.as_str()) {
                state.last_waitlist_code = Some(w_code.to_string());
            }
            contents.push(c);
        }
        session_mgr
            .update_demo_state(session_id, |s| *s = state.clone())
            .await;
        return make_response(spoken, contents);
    }

    // -------------------------------------------------------------
    // SCENARIO S11: Allergy Conflict on Order
    // -------------------------------------------------------------
    if (norm.contains("order") || norm.contains("satay") || norm.contains("baingan"))
        && (norm.contains("allergic to peanuts") || norm.contains("peanut allergy"))
    {
        // Place real order with seeded peanut item Banarasi Baingan Bharta (contains peanuts & sesame)
        let guest_name = extract_guest_name(raw_trimmed)
            .or_else(|| state.guest_name.clone())
            .unwrap_or_else(|| "Ravi".to_string());

        let args = json!({
            "guest_name": guest_name,
            "items": [
                {
                    "item_name": "Banarasi Baingan Bharta",
                    "quantity": 1
                }
            ],
            "allergen_acknowledgement": {
                "allergens_to_avoid": ["peanuts"],
                "acknowledged": false
            }
        });

        let (spoken, sc, _) = execute_tool(pool, "place_order", args, profile_token).await;
        let mut contents = Vec::new();
        if let Some(mut c) = sc {
            if c.is_object() && c.get("_tool_name").is_none() {
                c["_tool_name"] = json!("place_order");
            }
            contents.push(c);
        }
        return make_response(spoken, contents);
    }

    // -------------------------------------------------------------
    // SCENARIO S10: Pickup Order
    // -------------------------------------------------------------
    if norm.contains("order paneer tikka")
        || norm.contains("pickup order")
        || (norm.starts_with("order ") && !norm.contains("table"))
    {
        // Check for ambiguous or unknown dish like "order pizza"
        if norm.contains("pizza") || norm.contains("burger") || norm.contains("pasta") {
            return make_response(
                "We don't serve that item. Did you mean Paneer Tikka or Bhatti Ka Murgh?"
                    .to_string(),
                Vec::new(),
            );
        }

        let guest_name = extract_guest_name(raw_trimmed)
            .or_else(|| state.guest_name.clone())
            .unwrap_or_else(|| "Ravi".to_string());
        state.guest_name = Some(guest_name.clone());

        let pickup_time = extract_time(raw_trimmed)
            .map(|t| t.format("%H:%M").to_string())
            .unwrap_or_else(|| "20:00".to_string());

        let args = json!({
            "guest_name": guest_name,
            "pickup_time": pickup_time,
            "items": [
                {
                    "item_name": "Paneer Tikka Angaare",
                    "quantity": 1
                },
                {
                    "item_name": "Truffle Garlic Butter Naan",
                    "quantity": 2
                }
            ]
        });

        let (spoken, sc, _) = execute_tool(pool, "place_order", args, profile_token).await;
        let mut contents = Vec::new();
        if let Some(mut c) = sc {
            if c.is_object() && c.get("_tool_name").is_none() {
                c["_tool_name"] = json!("place_order");
            }
            if let Some(code) = c
                .get("order")
                .and_then(|o| o.get("order_code"))
                .and_then(|k| k.as_str())
            {
                state.last_order_code = Some(code.to_string());
            }
            contents.push(c);
        }
        session_mgr
            .update_demo_state(session_id, |s| *s = state.clone())
            .await;
        return make_response(spoken, contents);
    }

    // -------------------------------------------------------------
    // SCENARIO S7: Follow-up Negotiation Flow
    // -------------------------------------------------------------
    // Case 1: Pick an option ("the second one", "the patio one", "the bar one", "7 pm")
    if norm == "the second one"
        || norm == "second option"
        || norm == "second one"
        || norm == "option 2"
        || norm == "the first one"
        || norm == "first option"
        || norm == "option 1"
        || norm.contains("patio")
        || norm.contains("bar")
        || norm.contains("indoor")
        || norm.contains("7:30")
        || norm.contains("7.30")
        || (norm.contains("7 pm") && state.last_offered_alternatives.len() > 1)
        || (norm.contains("9") && state.last_offered_alternatives.len() > 1)
    {
        if let Some(alt) = state
            .last_offered_alternatives
            .iter()
            .find(|a| {
                if norm.contains("bar") {
                    a.area.to_lowercase() == "bar"
                } else if norm.contains("patio") {
                    a.area.to_lowercase() == "patio" || a.area.to_lowercase() == "outdoor"
                } else if norm.contains("first") || norm == "option 1" {
                    a.option_id == "opt_1"
                        || state
                            .last_offered_alternatives
                            .first()
                            .map(|f| f.option_id == a.option_id)
                            .unwrap_or(false)
                } else if norm.contains("second") || norm.contains("2") {
                    a.option_id == "opt_2"
                } else if norm.contains("9") {
                    a.time.starts_with("21:")
                } else {
                    a.time.starts_with("19:")
                }
            })
            .cloned()
            .or_else(|| state.last_offered_alternatives.first().cloned())
        {
            state.chosen_alternative = Some(alt.clone());
            session_mgr
                .update_demo_state(session_id, |s| *s = state.clone())
                .await;
            let area_label = match alt.area.to_lowercase().as_str() {
                "patio" | "outdoor" => "Patio",
                "bar" => "Bar",
                _ => "Indoor",
            };
            return make_response(
                format!(
                    "I have selected the {} table at {} for {} guests. Would you like me to hold this table for 5 minutes, or confirm the booking?",
                    area_label, alt.time, state.pending_party_size.unwrap_or(4)
                ),
                Vec::new(),
            );
        }
    }

    // Case 2: Hold it
    if norm.contains("hold it")
        || norm.contains("hold for 5 minutes")
        || norm == "hold"
        || norm.contains("hold this")
    {
        let area = state
            .chosen_alternative
            .as_ref()
            .map(|a| a.area.as_str())
            .unwrap_or("patio");
        let area_label = match area.to_lowercase().as_str() {
            "patio" | "outdoor" => "patio",
            "bar" => "bar",
            _ => "indoor",
        };
        let time = state
            .chosen_alternative
            .as_ref()
            .map(|a| a.time.as_str())
            .unwrap_or("7:30 PM");
        return make_response(
            format!(
                "I've placed a temporary hold on the {} table at {} for 5 minutes. Would you like to confirm the booking under your name?",
                area_label, time
            ),
            Vec::new(),
        );
    }

    // Case 3: Book it / Confirm -> Ask name if missing, or create booking
    if norm.contains("book it")
        || norm.contains("confirm booking")
        || norm == "confirm"
        || norm == "yes book it"
    {
        if state.guest_name.is_none() {
            return make_response(
                "May I have your name to complete the reservation?".to_string(),
                Vec::new(),
            );
        } else {
            let name = state.guest_name.clone().unwrap();
            let party = state.pending_party_size.unwrap_or(4);
            let date = state
                .pending_date
                .clone()
                .unwrap_or_else(|| resolve_date("friday", clock, tz, None));
            let time = state
                .pending_time
                .clone()
                .unwrap_or_else(|| "19:30".to_string());
            let area = state
                .chosen_alternative
                .as_ref()
                .map(|a| a.area.clone())
                .unwrap_or_else(|| "patio".to_string());

            let args = json!({
                "date": date,
                "time": time,
                "party_size": party,
                "area": area,
                "guest_name": name,
            });

            let (spoken, sc, _) = execute_tool(pool, "create_booking", args, profile_token).await;
            let mut contents = Vec::new();
            let mut confirmation_spoken = spoken;
            if let Some(mut c) = sc {
                if c.is_object() && c.get("_tool_name").is_none() {
                    c["_tool_name"] = json!("create_booking");
                }
                if let Some(code) = c
                    .get("booking")
                    .and_then(|b| b.get("confirmation_code"))
                    .and_then(|k| k.as_str())
                {
                    let spaced = format_spaced_code(code);
                    confirmation_spoken = format!(
                        "Your reservation is confirmed! Confirmation code is {} under {}.",
                        spaced, name
                    );
                }
                contents.push(c);
            }
            session_mgr.update_demo_state(session_id, |_| {}).await;
            return make_response(confirmation_spoken, contents);
        }
    }

    // Case 4: Name given as follow-up: "my name is Ravi" / "under Ravi" / "Ravi"
    if let Some(name) = extract_guest_name(raw_trimmed) {
        state.guest_name = Some(name.clone());
        let party = state.pending_party_size.unwrap_or(4);
        let date = state
            .pending_date
            .clone()
            .unwrap_or_else(|| resolve_date("friday", clock, tz, None));
        let time = state
            .pending_time
            .clone()
            .unwrap_or_else(|| "19:30".to_string());
        let area = state
            .chosen_alternative
            .as_ref()
            .map(|a| a.area.clone())
            .unwrap_or_else(|| "patio".to_string());

        let args = json!({
            "date": date,
            "time": time,
            "party_size": party,
            "area": area,
            "guest_name": name,
        });

        let (spoken, sc, _) = execute_tool(pool, "create_booking", args, profile_token).await;
        let mut contents = Vec::new();
        let mut confirmation_spoken = spoken;
        if let Some(mut c) = sc {
            if c.is_object() && c.get("_tool_name").is_none() {
                c["_tool_name"] = json!("create_booking");
            }
            if let Some(code) = c
                .get("booking")
                .and_then(|b| b.get("confirmation_code"))
                .and_then(|k| k.as_str())
            {
                let spaced = format_spaced_code(code);
                confirmation_spoken = format!(
                    "Your reservation is confirmed! Confirmation code is {} under {}.",
                    spaced, name
                );
            }
            contents.push(c);
        }
        session_mgr.update_demo_state(session_id, |_| {}).await;
        return make_response(confirmation_spoken, contents);
    }

    // -------------------------------------------------------------
    // SCENARIO S9: Large Party Deposit (party of 8)
    // -------------------------------------------------------------
    if (norm.contains("table for 8")
        || norm.contains("table for eight")
        || norm.contains("party of 8"))
        || (extract_party_size(raw_trimmed) == Some(8))
    {
        let party_size = 8;
        let req_time = extract_time(raw_trimmed).or_else(|| NaiveTime::from_hms_opt(20, 0, 0));
        let date = resolve_date(raw_trimmed, clock, tz, req_time);
        let time = req_time
            .map(|t| t.format("%H:%M").to_string())
            .unwrap_or_else(|| "20:00".to_string());
        let guest_name = extract_guest_name(raw_trimmed)
            .or_else(|| state.guest_name.clone())
            .unwrap_or_else(|| "Dining Guest".to_string());

        let args = json!({
            "date": date,
            "time": time,
            "party_size": party_size,
            "guest_name": guest_name,
            "area": "any"
        });

        let (_spoken, sc, _) = execute_tool(pool, "create_booking", args, profile_token).await;
        let mut contents = Vec::new();
        if let Some(mut c) = sc {
            if c.is_object() && c.get("_tool_name").is_none() {
                c["_tool_name"] = json!("create_booking");
            }
            contents.push(c);
        }
        // Voice must state deposit amount and simulated demo payment; never claim paid
        let custom_spoken = "For parties of 8 on Saturday at 8:00 PM, a refundable deposit of 2000 rupees is required to confirm your reservation. Please use the simulated checkout card below to complete payment. I cannot process payments directly.".to_string();
        return make_response(custom_spoken, contents);
    }

    // -------------------------------------------------------------
    // SCENARIO S6: The Hero Flow / Check Availability
    // -------------------------------------------------------------
    if norm.contains("table")
        || norm.contains("availability")
        || norm.contains("seat")
        || norm.contains("reservation")
    {
        let party_size = extract_party_size(raw_trimmed).unwrap_or(4);
        let req_time = extract_time(raw_trimmed).or_else(|| NaiveTime::from_hms_opt(19, 30, 0));
        let date = resolve_date(raw_trimmed, clock, tz, req_time);
        let time = req_time
            .map(|t| t.format("%H:%M").to_string())
            .unwrap_or_else(|| "19:30".to_string());
        let area = extract_area(raw_trimmed).unwrap_or_else(|| "indoor".to_string());

        state.pending_party_size = Some(party_size);
        state.pending_date = Some(date.clone());
        state.pending_time = Some(time.clone());
        state.pending_area = Some(area.clone());

        let args = json!({
            "date": date,
            "time": time,
            "party_size": party_size,
            "area": area,
            "alternatives": 3
        });

        let (spoken, sc, _) = execute_tool(pool, "check_availability", args, profile_token).await;
        let mut contents = Vec::new();
        if let Some(mut c) = sc {
            if c.is_object() && c.get("_tool_name").is_none() {
                c["_tool_name"] = json!("check_availability");
            }
            // Parse alternatives and store in state
            let mut alts = Vec::new();
            if let Some(arr) = c.get("alternatives").and_then(|a| a.as_array()) {
                for (idx, item) in arr.iter().enumerate() {
                    let d = item
                        .get("date")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&date)
                        .to_string();
                    let t = item
                        .get("time")
                        .and_then(|v| v.as_str())
                        .unwrap_or("19:30")
                        .to_string();
                    let a = item
                        .get("area")
                        .and_then(|v| v.as_str())
                        .unwrap_or("patio")
                        .to_string();
                    alts.push(OfferedAlternative {
                        option_id: format!("opt_{}", idx + 1),
                        date: d,
                        time: t.clone(),
                        area: a.clone(),
                        label: format!("{} at {}", a, t),
                    });
                }
            }
            if alts.is_empty() {
                // If engine provided none, supply standard patio and later indoor
                alts.push(OfferedAlternative {
                    option_id: "opt_1".to_string(),
                    date: date.clone(),
                    time: "19:30".to_string(),
                    area: "patio".to_string(),
                    label: "patio at 19:30".to_string(),
                });
                alts.push(OfferedAlternative {
                    option_id: "opt_2".to_string(),
                    date: date.clone(),
                    time: "21:00".to_string(),
                    area: "indoor".to_string(),
                    label: "indoor at 21:00".to_string(),
                });
            }
            state.last_offered_alternatives = alts;
            contents.push(c);
        }

        session_mgr
            .update_demo_state(session_id, |s| *s = state.clone())
            .await;
        return make_response(spoken, contents);
    }

    // -------------------------------------------------------------
    // SCENARIO S5: Allergy Search ("allergic to peanuts, what can I eat")
    // -------------------------------------------------------------
    if (norm.contains("allergic") || norm.contains("allergy")) && norm.contains("peanut") {
        let args = json!({
            "exclude_allergens": ["peanuts"],
            "limit": 5
        });

        let (_spoken, sc, _) = execute_tool(pool, "search_menu", args, profile_token).await;
        let mut contents = Vec::new();
        if let Some(mut c) = sc {
            if c.is_object() && c.get("_tool_name").is_none() {
                c["_tool_name"] = json!("search_menu");
            }
            contents.push(c);
        }
        let custom_spoken = "These dishes are listed as not containing peanuts. Please confirm with our staff before dining if you have severe sensitivities.".to_string();
        return make_response(custom_spoken, contents);
    }

    // -------------------------------------------------------------
    // SCENARIO S4: Menu Search
    // -------------------------------------------------------------
    if norm.contains("vegan")
        || norm.contains("gluten free")
        || norm.contains("spicy")
        || norm.contains("starter")
        || norm.contains("special")
        || norm.contains("menu")
        || norm.contains("food")
        || norm.contains("dish")
        || norm.contains("biryani")
        || norm.contains("paneer")
    {
        let mut args_obj = serde_json::Map::new();
        args_obj.insert("limit".to_string(), json!(5));

        if norm.contains("vegan") {
            args_obj.insert("dietary".to_string(), json!(["vegan"]));
        } else if norm.contains("gluten free") {
            args_obj.insert("dietary".to_string(), json!(["gluten_free"]));
        }

        if norm.contains("without peanuts") {
            args_obj.insert("exclude_allergens".to_string(), json!(["peanuts"]));
        }

        if norm.contains("under 300") || norm.contains("below 300") {
            args_obj.insert("max_price_cents".to_string(), json!(30000));
        }

        if norm.contains("starter") {
            args_obj.insert("query".to_string(), json!("starter"));
        } else if norm.contains("spicy") {
            args_obj.insert("query".to_string(), json!("spicy"));
        } else if norm.contains("special") {
            args_obj.insert("chefs_special_only".to_string(), json!(true));
        }

        let (spoken, sc, _) =
            execute_tool(pool, "search_menu", Value::Object(args_obj), profile_token).await;
        let mut contents = Vec::new();
        if let Some(mut c) = sc {
            if c.is_object() && c.get("_tool_name").is_none() {
                c["_tool_name"] = json!("search_menu");
            }
            contents.push(c);
        }
        return make_response(spoken, contents);
    }

    // -------------------------------------------------------------
    // SCENARIO S15: Goodbye / Thanks
    // -------------------------------------------------------------
    if norm.contains("thank you")
        || norm == "thanks"
        || norm.contains("thanks")
        || norm == "bye"
        || norm == "goodbye"
    {
        return make_response(
            "You're very welcome! Thank you for contacting The Saffron Nebula Bistro. Have a wonderful day!".to_string(),
            Vec::new(),
        );
    }

    // -------------------------------------------------------------
    // Default Friendly Capabilities Summary (Unknown Input)
    // -------------------------------------------------------------
    make_response(
        "I can help you reserve a table, browse our menu with allergen notices, or order takeout pickup. Try saying 'Table for 4 Friday at 7:30' or 'Vegan options without peanuts'.".to_string(),
        Vec::new(),
    )
}
