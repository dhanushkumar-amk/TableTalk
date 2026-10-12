use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub allowed_origins: Vec<String>,
    pub rust_log: String,
    pub request_timeout_secs: u64,
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

        Self {
            port,
            allowed_origins,
            rust_log,
            request_timeout_secs,
        }
    }
}
