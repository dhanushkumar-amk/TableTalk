use async_trait::async_trait;
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, InferenceConfiguration, Message, SystemContentBlock, Tool,
    ToolConfiguration, ToolInputSchema, ToolSpecification,
};
use aws_sdk_bedrockruntime::Client as SdkClient;
use aws_smithy_types::Document;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::orchestrator::types::{
    BedrockError, BedrockToolSpec, ChatMessage, ConverseOutput, TokenUsage, ToolUseRequest,
};

pub use crate::orchestrator::client::LlmClient;
pub use crate::orchestrator::client::LlmClient as BedrockClient;

/// Production AWS Bedrock Runtime client.
pub struct AwsBedrockClient {
    client: SdkClient,
}

impl AwsBedrockClient {
    pub async fn from_env(region: &str) -> Result<Self, BedrockError> {
        let region_provider = aws_config::Region::new(region.to_string());
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = SdkClient::new(&config);
        Ok(Self { client })
    }

    pub fn new(client: SdkClient) -> Self {
        Self { client }
    }
}

/// Lazy AWS Bedrock client that defers loading AWS SDK credentials until first request.
pub struct LazyAwsBedrockClient {
    region: String,
    inner: tokio::sync::OnceCell<SdkClient>,
}

impl LazyAwsBedrockClient {
    pub fn new(region: String) -> Self {
        Self {
            region,
            inner: tokio::sync::OnceCell::new(),
        }
    }

    async fn get_client(&self) -> Result<&SdkClient, BedrockError> {
        self.inner
            .get_or_try_init(|| async {
                let region_provider = aws_config::Region::new(self.region.clone());
                let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                    .region(region_provider)
                    .load()
                    .await;
                Ok(SdkClient::new(&config))
            })
            .await
    }
}

#[async_trait]
impl BedrockClient for LazyAwsBedrockClient {
    fn provider_name(&self) -> &'static str {
        "bedrock"
    }

    async fn converse(
        &self,
        model_id: &str,
        system_prompt: &str,
        messages: &[ChatMessage],
        tools: &[BedrockToolSpec],
        max_tokens: i32,
    ) -> Result<ConverseOutput, BedrockError> {
        let client = self.get_client().await?;
        AwsBedrockClient::new(client.clone())
            .converse(model_id, system_prompt, messages, tools, max_tokens)
            .await
    }
}

#[async_trait]
impl BedrockClient for AwsBedrockClient {
    fn provider_name(&self) -> &'static str {
        "bedrock"
    }

    async fn converse(
        &self,
        model_id: &str,
        system_prompt: &str,
        messages: &[ChatMessage],
        tools: &[BedrockToolSpec],
        max_tokens: i32,
    ) -> Result<ConverseOutput, BedrockError> {
        let mut builder = self.client.converse().model_id(model_id);

        // System prompt
        builder = builder.system(SystemContentBlock::Text(system_prompt.to_string()));

        // Inference configuration
        builder = builder.inference_config(
            InferenceConfiguration::builder()
                .max_tokens(max_tokens)
                .temperature(0.2)
                .build(),
        );

        // Messages
        let mut sdk_messages = Vec::new();
        for msg in messages {
            let role = match msg.role.as_str() {
                "assistant" => ConversationRole::Assistant,
                _ => ConversationRole::User,
            };
            let content = ContentBlock::Text(msg.content.clone());
            sdk_messages.push(
                Message::builder()
                    .role(role)
                    .content(content)
                    .build()
                    .map_err(|e| {
                        BedrockError::ModelError(format!("Failed to build message: {e}"))
                    })?,
            );
        }
        builder = builder.set_messages(Some(sdk_messages));

        // Tools
        if !tools.is_empty() {
            let mut sdk_tools = Vec::new();
            for t in tools {
                // Convert JSON schema to smithy Document
                let doc = json_to_document(&t.input_schema);
                let tool_spec = ToolSpecification::builder()
                    .name(&t.name)
                    .description(&t.description)
                    .input_schema(ToolInputSchema::Json(doc))
                    .build()
                    .map_err(|e| {
                        BedrockError::ModelError(format!("Failed to build tool spec: {e}"))
                    })?;
                sdk_tools.push(Tool::ToolSpec(tool_spec));
            }
            let tool_config = ToolConfiguration::builder()
                .set_tools(Some(sdk_tools))
                .build()
                .map_err(|e| {
                    BedrockError::ModelError(format!("Failed to build tool config: {e}"))
                })?;
            builder = builder.tool_config(tool_config);
        }

        let resp = builder.send().await.map_err(|sdk_err| {
            let err_str = format!("{sdk_err:?}");
            if err_str.contains("ThrottlingException")
                || err_str.contains("TooManyRequestsException")
            {
                BedrockError::Throttled(err_str)
            } else if err_str.contains("NoCredentials") || err_str.contains("CredentialsError") {
                BedrockError::NoCredentials(err_str)
            } else if err_str.contains("ServiceUnavailable") {
                BedrockError::Unavailable(err_str)
            } else {
                BedrockError::Other(err_str)
            }
        })?;

        // Parse Converse output
        let mut assistant_text = String::new();
        let mut tool_uses = Vec::new();

        if let Some(output) = resp.output() {
            if let Ok(msg) = output.as_message() {
                for block in msg.content() {
                    match block {
                        ContentBlock::Text(txt) => assistant_text.push_str(txt.as_str()),
                        ContentBlock::ToolUse(tu) => {
                            let input_val = document_to_json(tu.input());
                            tool_uses.push(ToolUseRequest {
                                tool_use_id: tu.tool_use_id().to_string(),
                                name: tu.name().to_string(),
                                input: input_val,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        let usage = resp.usage();
        let token_usage = TokenUsage {
            input_tokens: usage.map(|u| u.input_tokens()).unwrap_or(0),
            output_tokens: usage.map(|u| u.output_tokens()).unwrap_or(0),
            total_tokens: usage.map(|u| u.total_tokens()).unwrap_or(0),
        };

        let message = if !assistant_text.is_empty() || tool_uses.is_empty() {
            Some(ChatMessage {
                role: "assistant".to_string(),
                content: assistant_text,
            })
        } else {
            None
        };

        Ok(ConverseOutput {
            message,
            tool_uses,
            token_usage,
        })
    }
}

/// Convert serde_json::Value to aws_smithy_types::Document
pub fn json_to_document(val: &Value) -> Document {
    match val {
        Value::Null => Document::Null,
        Value::Bool(b) => Document::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Document::Number(aws_smithy_types::Number::PosInt(i as u64))
            } else if let Some(f) = n.as_f64() {
                Document::Number(aws_smithy_types::Number::Float(f))
            } else {
                Document::Null
            }
        }
        Value::String(s) => Document::String(s.clone()),
        Value::Array(arr) => Document::Array(arr.iter().map(json_to_document).collect()),
        Value::Object(map) => {
            let mut obj = std::collections::HashMap::new();
            for (k, v) in map {
                obj.insert(k.clone(), json_to_document(v));
            }
            Document::Object(obj)
        }
    }
}

/// Convert aws_smithy_types::Document to serde_json::Value
pub fn document_to_json(doc: &Document) -> Value {
    match doc {
        Document::Null => Value::Null,
        Document::Bool(b) => Value::Bool(*b),
        Document::Number(n) => match n {
            aws_smithy_types::Number::PosInt(u) => serde_json::json!(u),
            aws_smithy_types::Number::NegInt(i) => serde_json::json!(i),
            aws_smithy_types::Number::Float(f) => serde_json::json!(f),
        },
        Document::String(s) => Value::String(s.clone()),
        Document::Array(arr) => Value::Array(arr.iter().map(document_to_json).collect()),
        Document::Object(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                obj.insert(k.clone(), document_to_json(v));
            }
            Value::Object(obj)
        }
    }
}

/// Fake Bedrock client for deterministic unit and integration tests without AWS calls.
pub struct FakeBedrockClient {
    responses: Arc<Mutex<VecDeque<Result<ConverseOutput, BedrockError>>>>,
    pub recorded_payloads: Arc<Mutex<Vec<Vec<ChatMessage>>>>,
    pub recorded_system_prompts: Arc<Mutex<Vec<String>>>,
    pub provider: &'static str,
}

pub type FakeLlmClient = FakeBedrockClient;

impl Default for FakeBedrockClient {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeBedrockClient {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::new())),
            recorded_payloads: Arc::new(Mutex::new(Vec::new())),
            recorded_system_prompts: Arc::new(Mutex::new(Vec::new())),
            provider: "bedrock",
        }
    }

    pub fn with_provider(provider: &'static str) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::new())),
            recorded_payloads: Arc::new(Mutex::new(Vec::new())),
            recorded_system_prompts: Arc::new(Mutex::new(Vec::new())),
            provider,
        }
    }

    pub fn with_responses(responses: Vec<Result<ConverseOutput, BedrockError>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            recorded_payloads: Arc::new(Mutex::new(Vec::new())),
            recorded_system_prompts: Arc::new(Mutex::new(Vec::new())),
            provider: "bedrock",
        }
    }

    pub async fn push_response(&self, resp: Result<ConverseOutput, BedrockError>) {
        self.responses.lock().await.push_back(resp);
    }
}

#[async_trait]
impl BedrockClient for FakeBedrockClient {
    fn provider_name(&self) -> &'static str {
        self.provider
    }

    async fn converse(
        &self,
        _model_id: &str,
        system_prompt: &str,
        messages: &[ChatMessage],
        _tools: &[BedrockToolSpec],
        _max_tokens: i32,
    ) -> Result<ConverseOutput, BedrockError> {
        self.recorded_payloads.lock().await.push(messages.to_vec());
        self.recorded_system_prompts
            .lock()
            .await
            .push(system_prompt.to_string());

        let mut lock = self.responses.lock().await;
        if let Some(next_resp) = lock.pop_front() {
            next_resp
        } else {
            // Default response if queue empty
            Ok(ConverseOutput {
                message: Some(ChatMessage {
                    role: "assistant".to_string(),
                    content: "Hello! How can I help you today?".to_string(),
                }),
                tool_uses: Vec::new(),
                token_usage: TokenUsage {
                    input_tokens: 15,
                    output_tokens: 10,
                    total_tokens: 25,
                },
            })
        }
    }
}
