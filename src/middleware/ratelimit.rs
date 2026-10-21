//! A small in-memory rate limiter.
//!
//! It uses a fixed window per key. This is more than adequate for protecting
//! login and contact endpoints on a single-node deployment; a multi-node
//! deployment should replace it with a shared store.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::state::AppState;
use crate::util;

/// Configuration for a single rate-limited route.
#[derive(Debug, Clone, Copy)]
pub struct Limit {
    /// Maximum number of requests inside the window.
    pub requests: u32,
    /// Length of the window.
    pub window: Duration,
    /// Key used to scope the buckets.
    pub scope: &'static str,
}

impl Limit {
    pub const fn per_minute(requests: u32, scope: &'static str) -> Self {
        Self {
            requests,
            window: Duration::from_secs(60),
            scope,
        }
    }
    pub const fn per_hour(requests: u32, scope: &'static str) -> Self {
        Self {
            requests,
            window: Duration::from_secs(3600),
            scope,
        }
    }
}

/// Shared limiter state.
#[derive(Debug, Default)]
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, (u32, Instant)>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a hit. Returns `false` when the caller exceeded the limit.
    pub fn check(&self, key: &str, limit: Limit) -> bool {
        let mut buckets = match self.buckets.lock() {
            Ok(b) => b,
            Err(poisoned) => poisoned.into_inner(),
        };

        // Opportunistically prune expired buckets so the map cannot grow forever.
        if buckets.len() > 4096 {
            let now = Instant::now();
            buckets.retain(|_, (_, start)| now.duration_since(*start) < limit.window);
        }

        let now = Instant::now();
        let entry = buckets.entry(key.to_string()).or_insert((0, now));

        if now.duration_since(entry.1) >= limit.window {
            entry.0 = 0;
            entry.1 = now;
        }

        if entry.0 >= limit.requests {
            return false;
        }
        entry.0 += 1;
        true
    }

    /// Remove all state (used by tests).
    pub fn reset(&self) {
        if let Ok(mut buckets) = self.buckets.lock() {
            buckets.clear();
        }
    }
}

/// Middleware that applies a limit to every request, keyed by client IP.
pub async fn enforce(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();

    // Only throttle the endpoints that are actually expensive or abusable.
    let limit = if path.starts_with("/admin/login") {
        Some(Limit::per_minute(10, "login"))
    } else if path == "/contact" && request.method() == axum::http::Method::POST {
        Some(Limit::per_hour(20, "contact"))
    } else if path == "/newsletter/subscribe" {
        Some(Limit::per_hour(15, "newsletter"))
    } else if path.starts_with("/admin/api/markdown") {
        Some(Limit::per_minute(120, "markdown"))
    } else {
        None
    };

    let Some(limit) = limit else {
        return next.run(request).await;
    };

    let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(',').next().unwrap_or(v).trim().to_string())
        .or_else(|| {
            request
                .extensions()
                .get::<ConnectInfo<std::net::SocketAddr>>()
                .map(|info| info.0.ip().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let key = format!("{}:{}", limit.scope, ip);

    if state.limiter.check(&key, limit) {
        next.run(request).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            [(axum::http::header::RETRY_AFTER, "60")],
            "Too many requests. Please slow down and try again shortly.",
        )
            .into_response()
    }
}

/// Convenience helper used by handlers that need the caller's IP.
pub fn request_ip(headers: &axum::http::HeaderMap) -> String {
    util::client_ip(headers).unwrap_or_else(|| "unknown".to_string())
}
