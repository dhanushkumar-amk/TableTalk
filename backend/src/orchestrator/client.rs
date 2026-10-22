use async_trait::async_trait;

use crate::orchestrator::types::{ChatMessage, ConverseOutput, LlmError, ToolSpec};

/// Common trait for all LLM providers (Groq, Bedrock, and test doubles).
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Return the canonical identifier for this provider ("groq", "bedrock", etc.)
    fn provider_name(&self) -> &'static str;

    /// Execute a multi-turn conversational inference turn with optional tool specifications.
    async fn converse(
        &self,
        model_id: &str,
        system_prompt: &str,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
        max_tokens: i32,
    ) -> Result<ConverseOutput, LlmError>;
}
