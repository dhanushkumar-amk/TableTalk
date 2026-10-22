use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::orchestrator::client::LlmClient;
use crate::orchestrator::types::{
    ChatMessage, ConverseOutput, LlmError, TokenUsage, ToolSpec, ToolUseRequest,
};

/// Groq LLM API client implementing the shared LlmClient trait.
pub struct GroqClient {
    api_key: String,
    model: String,
    base_url: String,
    client: reqwest::Client,
}

impl GroqClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            base_url: "https://api.groq.com/openai/v1".to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn with_base_url(api_key: String, model: String, base_url: String) -> Self {
        Self {
            api_key,
            model,
            base_url,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl LlmClient for GroqClient {
    fn provider_name(&self) -> &'static str {
        "groq"
    }

    async fn converse(
        &self,
        model_id: &str,
        system_prompt: &str,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
        max_tokens: i32,
    ) -> Result<ConverseOutput, LlmError> {
        let trimmed_key = self.api_key.trim();
        if trimmed_key.is_empty() {
            return Err(LlmError::NoCredentials(
                "GROQ_API_KEY is not configured or empty".to_string(),
            ));
        }

        let effective_model = if model_id.is_empty() {
            &self.model
        } else {
            model_id
        };

        // Construct OpenAI-compatible messages array
        let mut api_messages = Vec::new();
        api_messages.push(json!({
            "role": "system",
            "content": system_prompt
        }));

        for msg in messages {
            api_messages.push(json!({
                "role": &msg.role,
                "content": &msg.content
            }));
        }

        let mut payload = json!({
            "model": effective_model,
            "messages": api_messages,
            "max_tokens": max_tokens,
            "temperature": 0.2
        });

        if !tools.is_empty() {
            let tool_definitions: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema
                        }
                    })
                })
                .collect();

            payload["tools"] = json!(tool_definitions);
            payload["tool_choice"] = json!("auto");
        }

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", trimmed_key)) {
            headers.insert(AUTHORIZATION, val);
        }

        // Retry loop for 429 (rate limits) and 5xx (transient errors) with backoff
        let mut attempts = 0;
        let max_attempts = 3;
        let mut backoff = std::time::Duration::from_millis(500);

        loop {
            attempts += 1;
            debug!(
                target: "orchestrator",
                model = %effective_model,
                attempt = attempts,
                "Sending request to Groq API"
            );

            let res_result = self
                .client
                .post(&url)
                .headers(headers.clone())
                .json(&payload)
                .send()
                .await;

            let response = match res_result {
                Ok(resp) => resp,
                Err(err) => {
                    if attempts < max_attempts && (err.is_timeout() || err.is_connect()) {
                        warn!(
                            target: "orchestrator",
                            attempt = attempts,
                            "Groq network error, retrying: {err}"
                        );
                        tokio::time::sleep(backoff).await;
                        backoff *= 2;
                        continue;
                    }
                    return Err(LlmError::Unavailable(format!("Groq network error: {err}")));
                }
            };

            let status = response.status();
            if status.is_success() {
                let json_body: Value = response.json().await.map_err(|e| {
                    LlmError::ModelError(format!("Failed to parse Groq response JSON: {e}"))
                })?;

                return parse_groq_response(json_body);
            }

            let status_code = status.as_u16();
            let body_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if status_code == 429 {
                if attempts < max_attempts {
                    warn!(
                        target: "orchestrator",
                        attempt = attempts,
                        "Groq rate limit 429 hit, backing off: {body_text}"
                    );
                    tokio::time::sleep(backoff).await;
                    backoff *= 2;
                    continue;
                }
                return Err(LlmError::Throttled(format!(
                    "Groq rate limit exceeded (429): {body_text}"
                )));
            }

            if status.is_server_error() {
                if attempts < max_attempts {
                    warn!(
                        target: "orchestrator",
                        attempt = attempts,
                        "Groq 5xx server error, backing off: {body_text}"
                    );
                    tokio::time::sleep(backoff).await;
                    backoff *= 2;
                    continue;
                }
                return Err(LlmError::Unavailable(format!(
                    "Groq server error ({status_code}): {body_text}"
                )));
            }

            if status_code == 401 || status_code == 403 {
                return Err(LlmError::NoCredentials(format!(
                    "Groq authentication failed ({status_code}): {body_text}"
                )));
            }

            return Err(LlmError::ModelError(format!(
                "Groq API error ({status_code}): {body_text}"
            )));
        }
    }
}

fn parse_groq_response(json: Value) -> Result<ConverseOutput, LlmError> {
    let choice = json
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| LlmError::ModelError("Missing 'choices' in Groq response".to_string()))?;

    let message = choice.get("message").cloned().unwrap_or(Value::Null);

    let text_content = message
        .get("content")
        .and_then(|c| c.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());

    let mut tool_uses = Vec::new();
    if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
        for call in tool_calls {
            let id = call
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("call_groq")
                .to_string();

            let func = call.get("function").cloned().unwrap_or(Value::Null);
            let name = func
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let input: Value = match func.get("arguments") {
                Some(Value::String(s)) => serde_json::from_str(s).unwrap_or(json!({})),
                Some(val @ Value::Object(_)) => val.clone(),
                _ => json!({}),
            };

            if !name.is_empty() {
                tool_uses.push(ToolUseRequest {
                    tool_use_id: id,
                    name,
                    input,
                });
            }
        }
    }

    let usage = json.get("usage");
    let input_tokens = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(|t| t.as_i64())
        .unwrap_or(0) as i32;
    let output_tokens = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|t| t.as_i64())
        .unwrap_or(0) as i32;
    let total_tokens = usage
        .and_then(|u| u.get("total_tokens"))
        .and_then(|t| t.as_i64())
        .unwrap_or(input_tokens as i64 + output_tokens as i64) as i32;

    let assistant_msg = text_content.map(|content| ChatMessage {
        role: "assistant".to_string(),
        content,
    });

    Ok(ConverseOutput {
        message: assistant_msg,
        tool_uses,
        token_usage: TokenUsage {
            input_tokens,
            output_tokens,
            total_tokens,
        },
    })
}
