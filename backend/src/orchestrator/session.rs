use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::orchestrator::router::ScriptedDemoState;
use crate::orchestrator::types::ToolUseRequest;

#[derive(Debug, Clone)]
pub struct SessionState {
    pub profile_token: Option<String>,
    pub tokens_used: usize,
    pub pending_destructive_tool: Option<ToolUseRequest>,
    pub last_active_utc: chrono::DateTime<chrono::Utc>,
    pub demo_state: ScriptedDemoState,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            profile_token: None,
            tokens_used: 0,
            pending_destructive_tool: None,
            last_active_utc: chrono::Utc::now(),
            demo_state: ScriptedDemoState::default(),
        }
    }
}

#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, SessionState>>>,
    global_daily_tokens: Arc<AtomicUsize>,
    session_token_budget: usize,
    global_token_budget: usize,
}

impl SessionManager {
    pub fn new(session_token_budget: usize, global_token_budget: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            global_daily_tokens: Arc::new(AtomicUsize::new(0)),
            session_token_budget,
            global_token_budget,
        }
    }

    pub async fn get_or_create(&self, session_id: &str) -> SessionState {
        let mut map = self.sessions.write().await;
        map.entry(session_id.to_string())
            .or_insert_with(SessionState::default)
            .clone()
    }

    pub async fn get_demo_state(&self, session_id: &str) -> ScriptedDemoState {
        let mut map = self.sessions.write().await;
        map.entry(session_id.to_string())
            .or_insert_with(SessionState::default)
            .demo_state
            .clone()
    }

    pub async fn update_demo_state<F>(&self, session_id: &str, f: F)
    where
        F: FnOnce(&mut ScriptedDemoState),
    {
        let mut map = self.sessions.write().await;
        let sess = map
            .entry(session_id.to_string())
            .or_insert_with(SessionState::default);
        f(&mut sess.demo_state);
        sess.last_active_utc = chrono::Utc::now();
    }

    pub async fn update_profile_token(&self, session_id: &str, token: String) {
        let mut map = self.sessions.write().await;
        if let Some(sess) = map.get_mut(session_id) {
            sess.profile_token = Some(token);
            sess.last_active_utc = chrono::Utc::now();
        }
    }

    pub async fn set_pending_destructive(&self, session_id: &str, tool: Option<ToolUseRequest>) {
        let mut map = self.sessions.write().await;
        if let Some(sess) = map.get_mut(session_id) {
            sess.pending_destructive_tool = tool;
            sess.last_active_utc = chrono::Utc::now();
        }
    }

    pub async fn record_tokens(&self, session_id: &str, tokens: usize) -> Result<(), &'static str> {
        let current_global = self.global_daily_tokens.fetch_add(tokens, Ordering::SeqCst);
        if current_global + tokens > self.global_token_budget {
            return Err("Global demo token budget reached for today. Please try again tomorrow.");
        }

        let mut map = self.sessions.write().await;
        let sess = map
            .entry(session_id.to_string())
            .or_insert_with(SessionState::default);

        sess.tokens_used += tokens;
        sess.last_active_utc = chrono::Utc::now();

        if sess.tokens_used > self.session_token_budget {
            return Err("Session demo token budget reached. Please start a new session or try again tomorrow.");
        }

        Ok(())
    }

    pub async fn check_budget(&self, session_id: &str) -> Result<(), &'static str> {
        if self.global_daily_tokens.load(Ordering::Relaxed) >= self.global_token_budget {
            return Err("Global demo token budget reached for today. Please try again tomorrow.");
        }

        let map = self.sessions.read().await;
        if let Some(sess) = map.get(session_id) {
            if sess.tokens_used >= self.session_token_budget {
                return Err("Session demo token budget reached. Please start a new session or try again tomorrow.");
            }
        }
        Ok(())
    }

    pub fn reset_daily_budget(&self) {
        self.global_daily_tokens.store(0, Ordering::SeqCst);
    }
}
