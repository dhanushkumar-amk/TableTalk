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
}

impl Config {
    pub fn from_env() -> Self {
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

        Self {
            port,
            allowed_origins,
            rust_log,
            request_timeout_secs,
            database_url,
            seed_on_start,
            restaurant_tz,
            rate_limit_per_minute,
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
        }
    }
}
