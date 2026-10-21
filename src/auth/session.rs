//! Database-backed session management.
//!
//! Sessions are opaque random identifiers stored in an `HttpOnly`, `SameSite=Lax`
//! cookie. The session row carries the CSRF token, so revoking a session
//! immediately invalidates both the login and any in-flight form.

use axum::http::HeaderMap;
use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;

use crate::config::Config;
use crate::error::AppResult;

pub const SESSION_COOKIE: &str = "ps_session";
pub const REMEMBER_COOKIE: &str = "ps_remember";

/// The authenticated user attached to a request, if any.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: i64,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub role: String,
    pub avatar: Option<String>,
    pub session_id: String,
    pub csrf_token: String,
}

impl AuthContext {
    pub fn is_owner(&self) -> bool {
        self.role == "owner"
    }
    pub fn can_manage_users(&self) -> bool {
        self.role == "owner"
    }
    pub fn is_admin(&self) -> bool {
        matches!(self.role.as_str(), "owner" | "editor")
    }
}

/// Create a new session for a user and return `(session_id, csrf_token)`.
pub async fn create_session(
    pool: &SqlitePool,
    config: &Config,
    user_id: i64,
    ip: Option<&str>,
    user_agent: Option<&str>,
    remember: bool,
) -> AppResult<(String, String, chrono::DateTime<Utc>)> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let csrf_token = uuid::Uuid::new_v4().to_string();
    let ttl = if remember {
        config.remember_ttl
    } else {
        config.session_ttl
    };
    let expires_at = Utc::now() + Duration::from_std(ttl).unwrap_or(Duration::hours(4));

    sqlx::query(
        "INSERT INTO sessions (id, user_id, csrf_token, expires_at, last_seen_at, ip, user_agent)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&session_id)
    .bind(user_id)
    .bind(&csrf_token)
    .bind(expires_at)
    .bind(Utc::now())
    .bind(ip)
    .bind(user_agent)
    .execute(pool)
    .await?;

    Ok((session_id, csrf_token, expires_at))
}

/// Look up the session referenced by the request cookies.
pub async fn load_session(
    pool: &SqlitePool,
    headers: &HeaderMap,
) -> AppResult<Option<AuthContext>> {
    let Some(session_id) = read_cookie(headers, SESSION_COOKIE) else {
        return Ok(None);
    };

    let row: Option<(i64, String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT s.user_id, s.csrf_token, u.role, s.expires_at
         FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.id = ? AND s.expires_at > ? AND u.is_active = 1",
    )
    .bind(&session_id)
    .bind(Utc::now())
    .fetch_optional(pool)
    .await?;

    let Some((user_id, csrf_token, role, _expires_at)) = row else {
        return Ok(None);
    };

    let (username, display_name, email, avatar): (String, String, String, Option<String>) =
        sqlx::query_as("SELECT username, display_name, email, avatar FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| crate::error::AppError::unauthorized("Session user no longer exists"))?;

    Ok(Some(AuthContext {
        user_id,
        username,
        display_name,
        email,
        role,
        avatar,
        session_id,
        csrf_token,
    }))
}

/// Touch the session so that active sessions do not expire.
pub async fn touch_session(pool: &SqlitePool, session_id: &str) {
    let _ = sqlx::query("UPDATE sessions SET last_seen_at = ? WHERE id = ?")
        .bind(Utc::now())
        .bind(session_id)
        .execute(pool)
        .await;
}

/// Delete a session (logout).
pub async fn destroy_session(pool: &SqlitePool, session_id: &str) {
    let _ = sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await;
}

/// Remove every session belonging to a user.
pub async fn destroy_user_sessions(pool: &SqlitePool, user_id: i64) {
    let _ = sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await;
}

/// Remove expired sessions; called periodically by the background worker.
pub async fn purge_expired_sessions(pool: &SqlitePool) -> u64 {
    sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
        .bind(Utc::now())
        .execute(pool)
        .await
        .map(|r| r.rows_affected())
        .unwrap_or(0)
}

/// Build the `Set-Cookie` header that stores the session.
pub fn session_cookie(session_id: &str, max_age: i64, secure: bool) -> Cookie<'static> {
    let mut cookie = Cookie::build((SESSION_COOKIE, session_id.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(max_age));
    if secure {
        cookie = cookie.secure(true);
    }
    cookie.build()
}

/// Build the `Set-Cookie` header that clears the session.
pub fn clear_session_cookie(secure: bool) -> Cookie<'static> {
    let mut cookie = Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::ZERO);
    if secure {
        cookie = cookie.secure(true);
    }
    cookie.build()
}

/// Read a cookie value from the request headers.
pub fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookies = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for part in cookies.split(';') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            if key.trim() == name {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// Validate a CSRF token against the session's token.
pub fn verify_csrf(session: &AuthContext, provided: &str) -> bool {
    use subtle::ConstantTimeEq;
    session
        .csrf_token
        .as_bytes()
        .ct_eq(provided.as_bytes())
        .into()
}
