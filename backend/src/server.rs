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

    /// Search the restaurant menu by dish name, ingredient, cuisine style, dietary needs, allergen exclusions, or budget.
    #[tool(
        description = "Search the restaurant menu by dish name, ingredient, cuisine style, dietary needs, allergen exclusions, or budget. Call this tool when the guest asks about food options, dishes, ingredients, or wants recommendations fitting dietary or allergen constraints. Do NOT call this tool for table reservations, checking table availability, placing orders, or checking opening hours.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::menu_search::SearchMenuOutput>(),
        annotations(
            read_only_hint = true,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn search_menu(
        &self,
        Parameters(input): Parameters<crate::tools::search_menu::SearchMenuInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::search_menu::handle_search_menu(pool, input).await;
        Ok(result)
    }

    /// Check dining table availability at the restaurant for a specific date, time, party size, and seating area.
    #[tool(
        description = "Check dining table availability at the restaurant for a specific date, time, party size, and seating area. Returns whether the requested slot is available with an allocated table, or suggests alternative nearby slots or dates if unavailable. Does NOT create a booking or hold a table. The caller must resolve relative date phrases like 'tomorrow' or 'next Friday' to a concrete ISO date (YYYY-MM-DD) in the restaurant timezone prior to calling.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::availability::CheckAvailabilityOutput>(),
        annotations(
            read_only_hint = true,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn check_availability(
        &self,
        Parameters(input): Parameters<crate::tools::check_availability::CheckAvailabilityInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::check_availability::handle_check_availability(pool, input).await;
        Ok(result)
    }

    /// Create a dining reservation at the restaurant.
    #[tool(
        description = "Create a dining reservation at the restaurant. Re-evaluates real-time table availability inside an immediate write transaction and assigns a best-fit dining table. Returns a confirmed reservation with a human-friendly confirmation code, or suggests alternatives if the slot became unavailable. Supports idempotency keys for safe retries.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::booking::CreateBookingOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_booking(
        &self,
        Parameters(input): Parameters<crate::tools::create_booking::CreateBookingInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::create_booking::handle_create_booking(pool, input).await;
        Ok(result)
    }

    /// Cancel an existing dining reservation using its confirmation code and guest name.
    #[tool(
        description = "Cancel an existing dining reservation using its confirmation code and primary guest name. Requires guest identity verification to prevent unauthorized cancellations. Cancelling an already cancelled reservation is idempotent.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::booking::CancelBookingOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = true,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn cancel_booking(
        &self,
        Parameters(input): Parameters<crate::tools::cancel_booking::CancelBookingInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::cancel_booking::handle_cancel_booking(pool, input).await;
        Ok(result)
    }

    /// Retrieve reservation details and summary by confirmation code and guest name.
    #[tool(
        description = "Retrieve reservation details and summary by confirmation code and primary guest name. Requires guest name matching for privacy.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::booking::GetBookingOutput>(),
        annotations(
            read_only_hint = true,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn get_booking(
        &self,
        Parameters(input): Parameters<crate::tools::get_booking::GetBookingInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::get_booking::handle_get_booking(pool, input).await;
        Ok(result)
    }

    /// Place a pickup order for food and beverages from the restaurant.
    #[tool(
        description = "Place a pickup order for food and beverages from the restaurant. Computes exact server-side pricing and tax, validates item availability, enforces kitchen preparation timing, and verifies safety-critical allergen restrictions. Returns a confirmed order with an order code (e.g. 'TO-8K4MZ') or reports conflicts and alternative dishes.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::ordering::PlaceOrderOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn place_order(
        &self,
        Parameters(input): Parameters<crate::tools::place_order::PlaceOrderInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::place_order::handle_place_order(pool, input).await;
        Ok(result)
    }

    /// Cancel an existing placed pickup order using its order code and guest name.
    #[tool(
        description = "Cancel an existing placed pickup order using its order code (e.g. 'TO-8K4MZ') and primary guest name. Requires guest identity verification. Allowed only prior to kitchen preparation.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::ordering::CancelOrderOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = true,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn cancel_order(
        &self,
        Parameters(input): Parameters<crate::tools::cancel_order::CancelOrderInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::cancel_order::handle_cancel_order(pool, input).await;
        Ok(result)
    }

    /// Retrieve pickup order details, line items, and preparation status using the order code and guest name.
    #[tool(
        description = "Retrieve pickup order details, line items, pricing breakdown, and preparation status using the order code and primary guest name. Requires guest name matching for privacy.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::ordering::GetOrderOutput>(),
        annotations(
            read_only_hint = true,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn get_order(
        &self,
        Parameters(input): Parameters<crate::tools::get_order::GetOrderInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::get_order::handle_get_order(pool, input).await;
        Ok(result)
    }

    /// Opt into personalized dining memory and profile storage.
    #[tool(
        description = "Opt into personalized dining memory and profile storage. Requires explicit guest consent ('consent_acknowledged: true'). Returns a secure bearer profile token ONCE which the caller must store securely. Never call without asking the guest first.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::memory::EnableMemoryOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn enable_memory(
        &self,
        Parameters(input): Parameters<crate::tools::enable_memory::EnableMemoryInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::enable_memory::handle_enable_memory(pool, input).await;
        Ok(result)
    }

    /// Retrieve the guest's saved dining profile, masked contact, preferences, and data export.
    #[tool(
        description = "Retrieve the guest's saved dining profile, masked contact information, preferences, counts, allergy reconfirmation status, and complete data export using their profile token.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::memory::GetProfileOutput>(),
        annotations(
            read_only_hint = true,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn get_profile(
        &self,
        Parameters(input): Parameters<crate::tools::get_profile::GetProfileInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::get_profile::handle_get_profile(pool, input).await;
        Ok(result)
    }

    /// Save or update a guest dining preference.
    #[tool(
        description = "Save or update a guest dining preference (allergy, dietary tag, seating preference, favorite menu item, or note). Values are validated against official registers and menu items. Deduplicates existing entries.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::memory::RememberPreferenceOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn remember_preference(
        &self,
        Parameters(input): Parameters<crate::tools::remember_preference::RememberPreferenceInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result =
            crate::tools::remember_preference::handle_remember_preference(pool, input).await;
        Ok(result)
    }

    /// Remove a saved preference from the guest profile.
    #[tool(
        description = "Remove a saved preference from the guest profile. Removing an allergy requires explicit confirmation ('confirm_remove_allergy: true') and alerts the guest that removal affects profile storage only.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::memory::ForgetPreferenceOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = true,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn forget_preference(
        &self,
        Parameters(input): Parameters<crate::tools::forget_preference::ForgetPreferenceInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::forget_preference::handle_forget_preference(pool, input).await;
        Ok(result)
    }

    /// Retrieve the guest's usual order suggestion.
    #[tool(
        description = "Retrieve the guest's 'usual order' suggestion computed deterministically from past order history. Re-validates items against live menu prices, stock availability, and profile allergies without placing an order.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::memory::GetUsualOrderOutput>(),
        annotations(
            read_only_hint = true,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn get_usual_order(
        &self,
        Parameters(input): Parameters<crate::tools::get_usual_order::GetUsualOrderInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::get_usual_order::handle_get_usual_order(pool, input).await;
        Ok(result)
    }

    /// Rotate the guest's profile bearer token atomically.
    #[tool(
        description = "Rotate the guest's profile bearer token atomically. Issues a new cryptographically random token and permanently revokes the previous token.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::memory::RotateProfileTokenOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn rotate_profile_token(
        &self,
        Parameters(input): Parameters<crate::tools::rotate_profile_token::RotateProfileTokenInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result =
            crate::tools::rotate_profile_token::handle_rotate_profile_token(pool, input).await;
        Ok(result)
    }

    /// Permanently hard-delete the guest profile and preferences ('Right to Be Forgotten').
    #[tool(
        description = "Permanently hard-delete the guest profile, consent records, tokens, and dining preferences ('Right to Be Forgotten'). Unlinks and anonymizes historical order and booking business records in one atomic transaction.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::memory::ForgetMeOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = true,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn forget_me(
        &self,
        Parameters(input): Parameters<crate::tools::forget_me::ForgetMeInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::forget_me::handle_forget_me(pool, input).await;
        Ok(result)
    }

    /// Negotiate dining options when preferred time or table is unavailable. Generates up to max_options ranked alternatives with explainable reason codes using deterministic integer scoring. Does NOT place a hold or create a booking. To hold or confirm an option, use accept_offer.
    #[tool(
        description = "Negotiate dining options when preferred time or table is unavailable. Generates up to max_options ranked alternatives with explainable reason codes using deterministic integer scoring. Does NOT place a hold or create a booking. To hold or confirm an option, use accept_offer.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::tools::negotiate_booking::NegotiateBookingOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn negotiate_booking(
        &self,
        Parameters(input): Parameters<crate::tools::negotiate_booking::NegotiateBookingInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::negotiate_booking::handle_negotiate_booking(pool, input).await;
        Ok(result)
    }

    /// Accept an option from an active negotiation or convert a previously held offer into a confirmed booking. When hold_only is true, places a short-lived hold (default 5 minutes) returning an offer_token. When hold_only is false or when offer_token is provided, completes the reservation.
    #[tool(
        description = "Accept an option from an active negotiation or convert a previously held offer into a confirmed booking. When hold_only is true, places a short-lived hold (default 5 minutes) returning an offer_token. When hold_only is false or when offer_token is provided, completes the reservation.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::tools::accept_offer::AcceptOfferOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn accept_offer(
        &self,
        Parameters(input): Parameters<crate::tools::accept_offer::AcceptOfferInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::accept_offer::handle_accept_offer(pool, input).await;
        Ok(result)
    }

    /// Release an active table hold, freeing the table for other guests or waitlist matching. Idempotent.
    #[tool(
        description = "Release an active table hold, freeing the table for other guests or waitlist matching. Idempotent.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::tools::release_offer::ReleaseOfferOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn release_offer(
        &self,
        Parameters(input): Parameters<crate::tools::release_offer::ReleaseOfferInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::release_offer::handle_release_offer(pool, input).await;
        Ok(result)
    }

    /// Join the dining waitlist for a specific date and time window when no tables are available. Returns a waitlist code and queue position. There are no push notifications; callers must poll get_waitlist_status.
    #[tool(
        description = "Join the dining waitlist for a specific date and time window when no tables are available. Returns a waitlist code and queue position. There are no push notifications; callers must poll get_waitlist_status.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::tools::join_waitlist::JoinWaitlistOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn join_waitlist(
        &self,
        Parameters(input): Parameters<crate::tools::join_waitlist::JoinWaitlistInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::join_waitlist::handle_join_waitlist(pool, input).await;
        Ok(result)
    }

    /// Check the status of a dining waitlist entry. Read-only. If a table opens up and is offered, returns an offer_token and hold expiry so the caller can call accept_offer.
    #[tool(
        description = "Check the status of a dining waitlist entry. Read-only. If a table opens up and is offered, returns an offer_token and hold expiry so the caller can call accept_offer.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::tools::get_waitlist_status::GetWaitlistStatusOutput>(),
        annotations(
            read_only_hint = true,
            idempotent_hint = true,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn get_waitlist_status(
        &self,
        Parameters(input): Parameters<crate::tools::get_waitlist_status::GetWaitlistStatusInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result =
            crate::tools::get_waitlist_status::handle_get_waitlist_status(pool, input).await;
        Ok(result)
    }

    /// Leave the dining waitlist and cancel any active offer hold. Idempotent.
    #[tool(
        description = "Leave the dining waitlist and cancel any active offer hold. Idempotent.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<crate::tools::leave_waitlist::LeaveWaitlistOutput>(),
        annotations(
            read_only_hint = false,
            idempotent_hint = true,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn leave_waitlist(
        &self,
        Parameters(input): Parameters<crate::tools::leave_waitlist::LeaveWaitlistInput>,
    ) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
        let pool = match &self.db {
            Some(p) => p,
            None => {
                return Ok(rmcp::model::CallToolResult::error(vec![
                    rmcp::model::ContentBlock::text(
                        "Database connection pool is not configured on TableTalkServer.",
                    ),
                ]));
            }
        };

        let result = crate::tools::leave_waitlist::handle_leave_waitlist(pool, input).await;
        Ok(result)
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
