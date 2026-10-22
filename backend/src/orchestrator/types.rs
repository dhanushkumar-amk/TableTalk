use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: Option<String>,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub total_tokens: i32,
}

impl TokenUsage {
    pub fn accumulate(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.total_tokens += other.total_tokens;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub session_id: String,
    pub message: ChatMessage,
    pub structured_contents: Vec<Value>,
    #[serde(default)]
    pub provider: String,
    pub fallback_mode: bool,
    pub token_usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

pub type ToolSpec = BedrockToolSpec;
pub type LlmError = BedrockError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolUseRequest {
    pub tool_use_id: String,
    pub name: String,
    pub input: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_use_id: String,
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConverseOutput {
    pub message: Option<ChatMessage>,
    pub tool_uses: Vec<ToolUseRequest>,
    pub token_usage: TokenUsage,
}

#[derive(Debug, thiserror::Error)]
pub enum BedrockError {
    #[error("Bedrock throttled: {0}")]
    Throttled(String),
    #[error("Bedrock unavailable: {0}")]
    Unavailable(String),
    #[error("Token budget exceeded: {0}")]
    BudgetExceeded(String),
    #[error("No AWS credentials: {0}")]
    NoCredentials(String),
    #[error("Bedrock model error: {0}")]
    ModelError(String),
    #[error("Bedrock error: {0}")]
    Other(String),
}
