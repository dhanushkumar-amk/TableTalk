use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use crate::availability::Clock;
pub use crate::orchestrator::bedrock::BedrockClient;
use crate::orchestrator::client::LlmClient;
use crate::orchestrator::executor::{build_tool_specs, execute_tool};
use crate::orchestrator::fallback::handle_fallback;
use crate::orchestrator::safety::{
    is_destructive_tool, is_explicit_confirmation, is_tool_allowed, redact_for_logging,
};
use crate::orchestrator::session::SessionManager;
use crate::orchestrator::system_prompt::generate_system_prompt;
use crate::orchestrator::types::{
    BedrockToolSpec, ChatMessage, ChatRequest, ChatResponse, TokenUsage,
};
use crate::{Config, TableTalkServer};

pub struct OrchestratorService {
    llm: Arc<dyn LlmClient>,
    session_manager: Arc<SessionManager>,
    pool: SqlitePool,
    _server: TableTalkServer,
    clock: Arc<dyn Clock>,
    config: Config,
    tool_specs: Vec<BedrockToolSpec>,
}

impl OrchestratorService {
    pub fn new(
        llm: Arc<dyn LlmClient>,
        session_manager: Arc<SessionManager>,
        pool: SqlitePool,
        server: TableTalkServer,
        clock: Arc<dyn Clock>,
        config: Config,
    ) -> Self {
        let tool_specs = build_tool_specs(&server);
        Self {
            llm,
            session_manager,
            pool,
            _server: server,
            clock,
            config,
            tool_specs,
        }
    }

    pub async fn process_chat(&self, req: ChatRequest) -> ChatResponse {
        let session_id = req
            .session_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Validate message count & length
        if req.messages.is_empty() {
            return ChatResponse {
                session_id,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: "Hello! How can I help you today?".to_string(),
                },
                structured_contents: Vec::new(),
                provider: self.llm.provider_name().to_string(),
                fallback_mode: false,
                token_usage: TokenUsage::default(),
            };
        }

        let last_user_msg = req
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.clone())
            .unwrap_or_default();

        if last_user_msg.len() > self.config.chat_max_message_len {
            return ChatResponse {
                session_id,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: format!(
                        "Your message exceeds the maximum allowed length of {} characters. Please shorten it.",
                        self.config.chat_max_message_len
                    ),
                },
                structured_contents: Vec::new(),
                provider: self.llm.provider_name().to_string(),
                fallback_mode: false,
                token_usage: TokenUsage::default(),
            };
        }

        // Restaurant local time
        let now_utc = self.clock.now();
        let tz: chrono_tz::Tz = self
            .config
            .restaurant_tz
            .parse()
            .unwrap_or(chrono_tz::Asia::Kolkata);
        let restaurant_now = now_utc.with_timezone(&tz).to_rfc3339();

        let session_state = self.session_manager.get_or_create(&session_id).await;
        let profile_token = session_state.profile_token.clone();

        // Check token budget before starting
        if let Err(budget_err) = self.session_manager.check_budget(&session_id).await {
            warn!(
                target: "orchestrator",
                session_id = %session_id,
                "Budget check failed: {}",
                budget_err
            );
            return handle_fallback(
                &self.pool,
                &self.session_manager,
                self.clock.as_ref(),
                tz,
                &session_id,
                &last_user_msg,
                profile_token.as_deref(),
            )
            .await;
        }

        // Check if there was a pending destructive tool awaiting confirmation
        let user_confirmed = is_explicit_confirmation(&last_user_msg);
        let pending_destructive = session_state.pending_destructive_tool.clone();

        // If user confirmed and there was a pending tool, we can execute it directly or proceed
        let mut structured_contents = Vec::new();
        if user_confirmed {
            if let Some(tool_req) = pending_destructive {
                self.session_manager
                    .set_pending_destructive(&session_id, None)
                    .await;
                let (res_text, sc, _) = execute_tool(
                    &self.pool,
                    &tool_req.name,
                    tool_req.input,
                    profile_token.as_deref(),
                )
                .await;
                if let Some(s) = sc {
                    structured_contents.push(s);
                }
                return ChatResponse {
                    session_id,
                    message: ChatMessage {
                        role: "assistant".to_string(),
                        content: res_text,
                    },
                    structured_contents,
                    provider: self.llm.provider_name().to_string(),
                    fallback_mode: false,
                    token_usage: TokenUsage::default(),
                };
            }
        }

        // Trim history to the last 8 messages
        let mut conversation: Vec<ChatMessage> = req
            .messages
            .iter()
            .rev()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        // Ensure no profile tokens leak in payloads sent to Bedrock
        for msg in &mut conversation {
            msg.content = crate::orchestrator::safety::redact_tokens(&msg.content);
        }

        let system_prompt = generate_system_prompt(&restaurant_now, &self.config.restaurant_tz);

        // Run orchestration loop with 20 second timeout and max 5 iterations
        let loop_result = tokio::time::timeout(
            Duration::from_secs(20),
            self.run_orchestration_loop(
                &session_id,
                &system_prompt,
                conversation,
                profile_token.as_deref(),
                user_confirmed,
            ),
        )
        .await;

        match loop_result {
            Ok(Ok(response)) => response,
            Ok(Err(err)) => {
                warn!(
                    target: "orchestrator",
                    session_id = %session_id,
                    provider = self.llm.provider_name(),
                    "LLM orchestration failed ({}), routing to fallback",
                    redact_for_logging(&err.to_string())
                );
                handle_fallback(
                    &self.pool,
                    &self.session_manager,
                    self.clock.as_ref(),
                    tz,
                    &session_id,
                    &last_user_msg,
                    profile_token.as_deref(),
                )
                .await
            }
            Err(_) => {
                warn!(
                    target: "orchestrator",
                    session_id = %session_id,
                    provider = self.llm.provider_name(),
                    "LLM orchestration timed out after 20s, routing to fallback"
                );
                handle_fallback(
                    &self.pool,
                    &self.session_manager,
                    self.clock.as_ref(),
                    tz,
                    &session_id,
                    &last_user_msg,
                    profile_token.as_deref(),
                )
                .await
            }
        }
    }

    async fn run_orchestration_loop(
        &self,
        session_id: &str,
        system_prompt: &str,
        mut conversation: Vec<ChatMessage>,
        profile_token: Option<&str>,
        user_confirmed: bool,
    ) -> Result<ChatResponse, crate::orchestrator::types::BedrockError> {
        let mut iterations = 0;
        let mut accumulated_tokens = TokenUsage::default();
        let mut structured_contents = Vec::new();
        let mut current_profile_token = profile_token.map(String::from);

        while iterations < 5 {
            iterations += 1;

            let model_id = match self.llm.provider_name() {
                "groq" => &self.config.groq_model,
                _ => &self.config.bedrock_model_id,
            };

            // Call LLM client
            let converse_out = self
                .llm
                .converse(
                    model_id,
                    system_prompt,
                    &conversation,
                    &self.tool_specs,
                    self.config.chat_max_tokens,
                )
                .await?;

            accumulated_tokens.accumulate(&converse_out.token_usage);

            // Record tokens against session and global budget
            if let Err(budget_err) = self
                .session_manager
                .record_tokens(session_id, converse_out.token_usage.total_tokens as usize)
                .await
            {
                return Err(crate::orchestrator::types::BedrockError::BudgetExceeded(
                    budget_err.to_string(),
                ));
            }

            // Log token usage per request without user message content
            info!(
                target: "orchestrator",
                session_id = %session_id,
                provider = self.llm.provider_name(),
                iteration = %iterations,
                input_tokens = %converse_out.token_usage.input_tokens,
                output_tokens = %converse_out.token_usage.output_tokens,
                total_tokens = %converse_out.token_usage.total_tokens,
                "LLM token usage recorded"
            );

            // If no tools requested, return final assistant answer
            if converse_out.tool_uses.is_empty() {
                let text = converse_out.message.map(|m| m.content).unwrap_or_else(|| {
                    "Thank you for contacting The Saffron Nebula Bistro.".to_string()
                });

                return Ok(ChatResponse {
                    session_id: session_id.to_string(),
                    message: ChatMessage {
                        role: "assistant".to_string(),
                        content: text,
                    },
                    structured_contents,
                    provider: self.llm.provider_name().to_string(),
                    fallback_mode: false,
                    token_usage: accumulated_tokens,
                });
            }

            // Process requested tools
            for tool_req in converse_out.tool_uses {
                if !is_tool_allowed(&tool_req.name) {
                    conversation.push(ChatMessage {
                        role: "user".to_string(),
                        content: format!(
                            "Error: Tool '{}' is not recognized or permitted.",
                            tool_req.name
                        ),
                    });
                    continue;
                }

                // Destructive tool confirmation check
                if is_destructive_tool(&tool_req.name) && !user_confirmed {
                    // Halt and ask user for confirmation
                    self.session_manager
                        .set_pending_destructive(session_id, Some(tool_req.clone()))
                        .await;

                    let confirm_prompt = format!(
                        "Are you sure you want to proceed with {}? Please confirm to proceed.",
                        tool_req.name.replace('_', " ")
                    );

                    return Ok(ChatResponse {
                        session_id: session_id.to_string(),
                        message: ChatMessage {
                            role: "assistant".to_string(),
                            content: confirm_prompt,
                        },
                        structured_contents,
                        provider: self.llm.provider_name().to_string(),
                        fallback_mode: false,
                        token_usage: accumulated_tokens,
                    });
                }

                // Execute tool in-process
                let tool_input = tool_req.input.clone();
                let (tool_res_text, sc, is_error) = execute_tool(
                    &self.pool,
                    &tool_req.name,
                    tool_input,
                    current_profile_token.as_deref(),
                )
                .await;

                if let Some(mut struct_val) = sc {
                    // Check if token was issued by enable_memory or rotate_profile_token
                    if let Some(tok) = struct_val.get("profile_token").and_then(|v| v.as_str()) {
                        self.session_manager
                            .update_profile_token(session_id, tok.to_string())
                            .await;
                        current_profile_token = Some(tok.to_string());
                    }
                    if struct_val.is_object() && struct_val.get("_tool_name").is_none() {
                        struct_val["_tool_name"] = serde_json::json!(tool_req.name);
                    }
                    structured_contents.push(struct_val);
                }

                // Redact tokens before feeding back into conversation
                let safe_tool_text = crate::orchestrator::safety::redact_tokens(&tool_res_text);

                if is_error {
                    // Feed error back for self-correction
                    conversation.push(ChatMessage {
                        role: "user".to_string(),
                        content: format!(
                            "Tool execution error for '{}': {safe_tool_text}",
                            tool_req.name
                        ),
                    });
                } else {
                    conversation.push(ChatMessage {
                        role: "user".to_string(),
                        content: format!("Tool '{}' result: {safe_tool_text}", tool_req.name),
                    });
                }
            }
        }

        // Max iterations reached
        Ok(ChatResponse {
            session_id: session_id.to_string(),
            message: ChatMessage {
                role: "assistant".to_string(),
                content:
                    "I've reached the maximum steps for this request. How else can I assist you?"
                        .to_string(),
            },
            structured_contents,
            provider: self.llm.provider_name().to_string(),
            fallback_mode: false,
            token_usage: accumulated_tokens,
        })
    }
}
