use crate::tools::{handle_ping, PingInput, PingOutput};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router, ServerHandler,
};

/// The TableTalk MCP Server implementation.
/// Handles protocol lifecycle, capabilities declaration, and tool routing.
#[derive(Clone)]
pub struct TableTalkServer {
    tool_router: ToolRouter<Self>,
    pub db: Option<sqlx::SqlitePool>,
}

impl Default for TableTalkServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl TableTalkServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            db: None,
        }
    }

    pub fn with_db(db: sqlx::SqlitePool) -> Self {
        Self {
            tool_router: Self::tool_router(),
            db: Some(db),
        }
    }

    /// Ping the TableTalk restaurant assistant to verify connectivity and get server metadata.
    #[tool(
        description = "Ping the TableTalk restaurant assistant to verify connectivity and retrieve server status",
        annotations(
            read_only_hint = true,
            idempotent_hint = true,
            destructive_hint = false
        )
    )]
    async fn ping_restaurant(
        &self,
        Parameters(input): Parameters<PingInput>,
    ) -> Result<rmcp::Json<PingOutput>, String> {
        let output = handle_ping(input);
        Ok(rmcp::Json(output))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for TableTalkServer {
    fn get_info(&self) -> InitializeResult {
        let mut info = InitializeResult::new(ServerCapabilities::builder().enable_tools().build());

        let mut server_info = Implementation::new("tabletalk", env!("CARGO_PKG_VERSION"));
        server_info.title = Some("TableTalk Restaurant MCP Server".to_string());
        server_info.description = Some(
            "TableTalk is an intelligent restaurant assistant MCP server that powers dining interactions."
                .to_string(),
        );

        info.server_info = server_info;
        info.instructions = Some(
            "TableTalk is an intelligent restaurant assistant powered by the Model Context Protocol. \
            It enables voice assistants and client interfaces to discover menu items, inspect dietary tags, \
            check real-time dining room availability, confirm reservations, and manage orders."
                .to_string(),
        );

        info
    }
}
