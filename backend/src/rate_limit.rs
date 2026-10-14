use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const DEFAULT_RATE_WINDOW: Duration = Duration::from_secs(60);

/// In-memory sliding-window rate limiter per client IP.
#[derive(Debug)]
pub struct IpRateLimiter {
    per_ip: Mutex<HashMap<String, VecDeque<Instant>>>,
    pub limit_per_window: usize,
    pub window: Duration,
}

impl IpRateLimiter {
    pub fn new(limit_per_window: usize) -> Self {
        Self {
            per_ip: Mutex::new(HashMap::new()),
            limit_per_window,
            window: DEFAULT_RATE_WINDOW,
        }
    }

    pub fn with_window(limit_per_window: usize, window: Duration) -> Self {
        Self {
            per_ip: Mutex::new(HashMap::new()),
            limit_per_window,
            window,
        }
    }

    /// Check if a request from the given IP is allowed.
    /// If allowed, records the request and returns `Ok(())`.
    /// If exceeded, returns `Err(retry_after_secs)`.
    pub fn check(&self, ip: &str) -> Result<(), u64> {
        if self.limit_per_window == 0 {
            // 0 means unlimited
            return Ok(());
        }

        let now = Instant::now();
        let mut guard = self.per_ip.lock().unwrap();
        let entry = guard.entry(ip.to_string()).or_default();

        // Evict timestamps older than window
        while let Some(front) = entry.front() {
            if now.duration_since(*front) > self.window {
                entry.pop_front();
            } else {
                break;
            }
        }

        if entry.len() >= self.limit_per_window {
            let oldest = *entry.front().unwrap();
            let elapsed = now.duration_since(oldest);
            let retry_after = if elapsed < self.window {
                (self.window - elapsed).as_secs().max(1)
            } else {
                1
            };
            Err(retry_after)
        } else {
            entry.push_back(now);
            Ok(())
        }
    }
}

/// Helper to extract the client IP string from HTTP headers (X-Forwarded-For, X-Real-IP)
/// or default to "127.0.0.1".
pub fn extract_client_ip(req: &Request) -> String {
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(s) = forwarded.to_str() {
            if let Some(first) = s.split(',').next() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(s) = real_ip.to_str() {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

/// Axum middleware function that enforces IP rate limiting.
pub async fn rate_limit_middleware(
    State(limiter): State<Arc<IpRateLimiter>>,
    req: Request,
    next: Next,
) -> Response {
    let client_ip = extract_client_ip(&req);
    match limiter.check(&client_ip) {
        Ok(()) => next.run(req).await,
        Err(retry_after) => {
            let mut response = (
                StatusCode::TOO_MANY_REQUESTS,
                "Too Many Requests: Rate limit exceeded. Please wait before retrying.\n",
            )
                .into_response();
            if let Ok(val) = HeaderValue::from_str(&retry_after.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, val);
            }
            response
        }
    }
}
