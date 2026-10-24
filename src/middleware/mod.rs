//! Cross-cutting middleware.

pub mod ratelimit;
pub mod security;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::session;
use crate::state::AppState;
use crate::util;

/// Detect a canonical language prefix (`/en/` or `/fa/`) in the URL and record
/// it on a custom header so `context::build` can force that locale.
pub async fn resolve_locale_prefix(mut request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let locale = if path.starts_with("/en/") || path == "/en" {
        Some("en")
    } else if path.starts_with("/fa/") || path == "/fa" {
        Some("fa")
    } else {
        None
    };
    if let Some(locale) = locale {
        if let Ok(value) = axum::http::HeaderValue::from_str(locale) {
            request.headers_mut().insert("x-locale", value);
        }
    }
    next.run(request).await
}

/// Load the session (if any) and attach it to the request extensions.
pub async fn load_session(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    match session::load_session(&state.db, request.headers()).await {
        Ok(Some(auth)) => {
            session::touch_session(&state.db, &auth.session_id).await;
            request.extensions_mut().insert(auth);
        }
        Ok(None) => {}
        Err(err) => {
            tracing::warn!(error = %err, "failed to load session");
        }
    }
    next.run(request).await
}

/// Record a page view for the built-in analytics.
///
/// The write happens on a detached task so the response is never delayed.
pub async fn track_view(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let referrer = request
        .headers()
        .get(axum::http::header::REFERER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().take(500).collect::<String>());
    let agent = util::user_agent(request.headers());
    let ip = util::client_ip(request.headers());

    let response = next.run(request).await;

    let is_page = method == axum::http::Method::GET
        && response.status().is_success()
        && !path.starts_with("/admin")
        && !path.starts_with("/assets")
        && !path.starts_with("/media")
        && !path.starts_with("/favicon")
        && path != "/robots.txt"
        && path != "/manifest.webmanifest"
        && !path.ends_with(".xml")
        && !path.ends_with(".css")
        && !path.ends_with(".js");

    if is_page {
        let db = state.db.clone();
        let path = path.clone();
        let referrer = referrer.clone();
        let agent = agent.clone();
        let ip_hash = ip.map(|ip| util::hash_ip(&ip, &state.config.secret));

        tokio::spawn(async move {
            let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let result = sqlx::query(
                "INSERT INTO page_views (path, referrer, user_agent, country, day, created_at)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(&path)
            .bind(&referrer)
            .bind(&agent)
            .bind(ip_hash)
            .bind(&day)
            .bind(chrono::Utc::now())
            .execute(&db)
            .await;

            if let Err(err) = result {
                tracing::debug!(error = %err, "failed to record page view");
            }
        });
    }

    response
}
