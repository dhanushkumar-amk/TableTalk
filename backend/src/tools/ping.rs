use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Input parameters for the `ping_restaurant` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct PingInput {
    /// Optional guest or client name to include in greeting
    pub name: Option<String>,
    /// Optional simulation delay in milliseconds to test client/server timeout behavior
    pub sleep_ms: Option<u64>,
}

/// Structured response from the `ping_restaurant` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PingOutput {
    /// Friendly greeting message confirming assistant status
    pub message: String,
    /// Identifier of the serving MCP implementation
    pub server: String,
    /// Crate package version of the TableTalk server
    pub version: String,
}

/// Business logic for executing the hello-world ping tool.
pub fn handle_ping(input: PingInput) -> PingOutput {
    let guest = input
        .name
        .as_deref()
        .map(|n| {
            let cleaned: String = n.chars().filter(|c| !c.is_control()).collect();
            let trimmed = cleaned.trim();
            if trimmed.is_empty() {
                "Guest".to_string()
            } else {
                trimmed.chars().take(100).collect::<String>()
            }
        })
        .unwrap_or_else(|| "Guest".to_string());
    PingOutput {
        message: format!(
            "Hello, {}! TableTalk restaurant MCP server is online.",
            guest
        ),
        server: "tabletalk".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_ping_default() {
        let output = handle_ping(PingInput {
            name: None,
            sleep_ms: None,
        });
        assert_eq!(
            output.message,
            "Hello, Guest! TableTalk restaurant MCP server is online."
        );
        assert_eq!(output.server, "tabletalk");
        assert_eq!(output.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_handle_ping_named() {
        let output = handle_ping(PingInput {
            name: Some("Alice".to_string()),
            sleep_ms: None,
        });
        assert_eq!(
            output.message,
            "Hello, Alice! TableTalk restaurant MCP server is online."
        );
    }
}
