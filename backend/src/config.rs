use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub allowed_origins: Vec<String>,
    pub rust_log: String,
    pub request_timeout_secs: u64,
    pub database_url: String,
    pub seed_on_start: bool,
    pub restaurant_tz: String,
    pub rate_limit_per_minute: usize,
    pub aws_region: String,
    pub bedrock_model_id: String,
    pub chat_max_tokens: i32,
    pub chat_session_token_budget: usize,
    pub chat_daily_token_budget: usize,
    pub chat_rate_limit_per_minute: usize,
    pub chat_max_message_len: usize,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let port: u16 = env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let allowed_origins = env::var("ALLOWED_ORIGINS")
            .ok()
            .map(|val| {
                val.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![
                    "http://localhost:5173".to_string(),
                    "http://127.0.0.1:5173".to_string(),
                    "http://localhost:8080".to_string(),
                    "http://127.0.0.1:8080".to_string(),
                ]
            });

        let rust_log =
            env::var("RUST_LOG").unwrap_or_else(|_| "info,tabletalk_backend=debug".to_string());

        let request_timeout_secs: u64 = env::var("REQUEST_TIMEOUT_SECS")
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(30);

        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/tabletalk.db".to_string());

        let seed_on_start = env::var("SEED_ON_START")
            .ok()
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let restaurant_tz =
            env::var("RESTAURANT_TZ").unwrap_or_else(|_| "Asia/Kolkata".to_string());

        let rate_limit_per_minute: usize = env::var("RATE_LIMIT_PER_MINUTE")
            .ok()
            .and_then(|r| r.parse().ok())
            .unwrap_or(600);

        let aws_region = env::var("AWS_REGION").unwrap_or_else(|_| "ap-south-1".to_string());

        let bedrock_model_id = env::var("BEDROCK_MODEL_ID")
            .unwrap_or_else(|_| "apac.amazon.nova-lite-v1:0".to_string());

        let chat_max_tokens: i32 = env::var("CHAT_MAX_TOKENS")
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(500);

        let chat_session_token_budget: usize = env::var("CHAT_SESSION_TOKEN_BUDGET")
            .ok()
            .and_then(|b| b.parse().ok())
            .unwrap_or(10000);

        let chat_daily_token_budget: usize = env::var("CHAT_DAILY_TOKEN_BUDGET")
            .ok()
            .and_then(|b| b.parse().ok())
            .unwrap_or(50000);

        let chat_rate_limit_per_minute: usize = env::var("CHAT_RATE_LIMIT_PER_MINUTE")
            .ok()
            .and_then(|r| r.parse().ok())
            .unwrap_or(30);

        let chat_max_message_len: usize = env::var("CHAT_MAX_MESSAGE_LEN")
            .ok()
            .and_then(|m| m.parse().ok())
            .unwrap_or(1000);

        Self {
            port,
            allowed_origins,
            rust_log,
            request_timeout_secs,
            database_url,
            seed_on_start,
            restaurant_tz,
            rate_limit_per_minute,
            aws_region,
            bedrock_model_id,
            chat_max_tokens,
            chat_session_token_budget,
            chat_daily_token_budget,
            chat_rate_limit_per_minute,
            chat_max_message_len,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 8080,
            allowed_origins: vec![
                "http://localhost:5173".to_string(),
                "http://127.0.0.1:5173".to_string(),
                "http://localhost:8080".to_string(),
                "http://127.0.0.1:8080".to_string(),
            ],
            rust_log: "info,tabletalk_backend=debug".to_string(),
            request_timeout_secs: 30,
            database_url: "sqlite::memory:".to_string(),
            seed_on_start: false,
            restaurant_tz: "Asia/Kolkata".to_string(),
            rate_limit_per_minute: 600,
            aws_region: "ap-south-1".to_string(),
            bedrock_model_id: "apac.amazon.nova-lite-v1:0".to_string(),
            chat_max_tokens: 500,
            chat_session_token_budget: 10000,
            chat_daily_token_budget: 50000,
            chat_rate_limit_per_minute: 30,
            chat_max_message_len: 1000,
        }
    }
}
