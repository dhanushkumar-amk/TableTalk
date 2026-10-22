use std::sync::Arc;

use chrono_tz::Tz;
use sqlx::SqlitePool;

use crate::availability::Clock;
use crate::orchestrator::router::handle_scripted_chat;
use crate::orchestrator::session::SessionManager;
use crate::orchestrator::types::ChatResponse;

/// Scripted demo router invoked when LLM is unavailable, unconfigured, or in demo fallback mode.
pub async fn handle_fallback(
    pool: &SqlitePool,
    session_mgr: &Arc<SessionManager>,
    clock: &dyn Clock,
    tz: Tz,
    session_id: &str,
    last_user_message: &str,
    profile_token: Option<&str>,
) -> ChatResponse {
    handle_scripted_chat(
        pool,
        session_mgr,
        clock,
        tz,
        session_id,
        last_user_message,
        profile_token,
    )
    .await
}
