use std::env;

#[derive(Clone)]
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
    pub llm_provider: String,
    pub groq_api_key: Option<String>,
    pub groq_model: String,
}

fn parse_env_var<T: std::str::FromStr>(name: &str, default: T) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    match env::var(name) {
        Ok(val) => {
            let trimmed = val.trim();
            if trimmed.is_empty() {
                Ok(default)
            } else {
                trimmed
                    .parse::<T>()
                    .map_err(|e| format!("Invalid environment variable {}='{}': {}", name, val, e))
            }
        }
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => Err(format!(
            "Environment variable {} contains invalid unicode",
            name
        )),
    }
}

impl Config {
    /// Attempts to load configuration from environment, validating types and ranges.
    /// Fails fast with descriptive error messages on malformed configuration.
    pub fn try_from_env() -> Result<Self, String> {
        dotenvy::dotenv().ok();

        let port: u16 = parse_env_var("PORT", 8080)?;
        if port == 0 {
            return Err("PORT must be greater than 0".to_string());
        }

        let allowed_origins = env::var("ALLOWED_ORIGINS")
            .ok()
            .map(|val| {
                val.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|list| !list.is_empty())
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

        let request_timeout_secs: u64 = parse_env_var("REQUEST_TIMEOUT_SECS", 30)?;
        if request_timeout_secs == 0 {
            return Err("REQUEST_TIMEOUT_SECS must be at least 1".to_string());
        }

        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/tabletalk.db".to_string());
        if database_url.trim().is_empty() {
            return Err("DATABASE_URL must not be empty".to_string());
        }

        let seed_on_start = env::var("SEED_ON_START")
            .ok()
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let restaurant_tz =
            env::var("RESTAURANT_TZ").unwrap_or_else(|_| "Asia/Kolkata".to_string());

        let rate_limit_per_minute: usize = parse_env_var("RATE_LIMIT_PER_MINUTE", 600)?;
        if rate_limit_per_minute == 0 {
            return Err("RATE_LIMIT_PER_MINUTE must be at least 1".to_string());
        }

        let aws_region = env::var("AWS_REGION").unwrap_or_else(|_| "ap-south-1".to_string());

        let bedrock_model_id = env::var("BEDROCK_MODEL_ID")
            .unwrap_or_else(|_| "apac.amazon.nova-lite-v1:0".to_string());

        let chat_max_tokens: i32 = parse_env_var("CHAT_MAX_TOKENS", 500)?;
        if chat_max_tokens <= 0 {
            return Err("CHAT_MAX_TOKENS must be greater than 0".to_string());
        }

        let chat_session_token_budget: usize = parse_env_var("CHAT_SESSION_TOKEN_BUDGET", 10000)?;
        if chat_session_token_budget == 0 {
            return Err("CHAT_SESSION_TOKEN_BUDGET must be at least 1".to_string());
        }

        let chat_daily_token_budget: usize = parse_env_var("CHAT_DAILY_TOKEN_BUDGET", 50000)?;
        if chat_daily_token_budget == 0 {
            return Err("CHAT_DAILY_TOKEN_BUDGET must be at least 1".to_string());
        }

        let chat_rate_limit_per_minute: usize = parse_env_var("CHAT_RATE_LIMIT_PER_MINUTE", 30)?;
        if chat_rate_limit_per_minute == 0 {
            return Err("CHAT_RATE_LIMIT_PER_MINUTE must be at least 1".to_string());
        }

        let chat_max_message_len: usize = parse_env_var("CHAT_MAX_MESSAGE_LEN", 1000)?;
        if chat_max_message_len == 0 {
            return Err("CHAT_MAX_MESSAGE_LEN must be at least 1".to_string());
        }

        let llm_provider = env::var("LLM_PROVIDER")
            .unwrap_or_else(|_| "groq".to_string())
            .to_lowercase();

        if llm_provider != "groq" && llm_provider != "bedrock" {
            return Err(format!(
                "Invalid LLM_PROVIDER '{}': must be 'groq' or 'bedrock'",
                llm_provider
            ));
        }

        let groq_api_key = env::var("GROQ_API_KEY")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let groq_model =
            env::var("GROQ_MODEL").unwrap_or_else(|_| "llama-3.3-70b-versatile".to_string());

        Ok(Self {
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
            llm_provider,
            groq_api_key,
            groq_model,
        })
    }

    pub fn from_env() -> Self {
        Self::try_from_env().unwrap_or_else(|err| {
            eprintln!("Configuration Error: {}", err);
            std::process::exit(1);
        })
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("port", &self.port)
            .field("allowed_origins", &self.allowed_origins)
            .field("rust_log", &self.rust_log)
            .field("request_timeout_secs", &self.request_timeout_secs)
            .field("database_url", &self.database_url)
            .field("seed_on_start", &self.seed_on_start)
            .field("restaurant_tz", &self.restaurant_tz)
            .field("rate_limit_per_minute", &self.rate_limit_per_minute)
            .field("aws_region", &self.aws_region)
            .field("bedrock_model_id", &self.bedrock_model_id)
            .field("chat_max_tokens", &self.chat_max_tokens)
            .field("chat_session_token_budget", &self.chat_session_token_budget)
            .field("chat_daily_token_budget", &self.chat_daily_token_budget)
            .field(
                "chat_rate_limit_per_minute",
                &self.chat_rate_limit_per_minute,
            )
            .field("chat_max_message_len", &self.chat_max_message_len)
            .field("llm_provider", &self.llm_provider)
            .field(
                "groq_api_key",
                &self.groq_api_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("groq_model", &self.groq_model)
            .finish()
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
            llm_provider: "groq".to_string(),
            groq_api_key: None,
            groq_model: "llama-3.3-70b-versatile".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_redaction_in_debug() {
        let config = Config {
            groq_api_key: Some("gsk_secret1234567890abcdef".to_string()),
            ..Config::default()
        };
        let debug_str = format!("{:?}", config);
        assert!(!debug_str.contains("gsk_secret1234567890abcdef"));
        assert!(debug_str.contains("[REDACTED]"));
    }

    #[test]
    fn test_default_config_valid() {
        let config = Config::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.llm_provider, "groq");
    }

    #[test]
    fn test_try_from_env_loads_successfully() {
        let res = Config::try_from_env();
        assert!(
            res.is_ok(),
            "Config::try_from_env should succeed: {:?}",
            res.err()
        );
    }
}
