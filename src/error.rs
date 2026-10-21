//! Central error type. Every handler returns `Result<..., AppError>` and the
//! error is rendered either as HTML (browser) or JSON (API clients).

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    Validation(String),
    RateLimited(String),
    Database(sqlx::Error),
    Internal(String),
}

impl AppError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized(msg.into())
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::Forbidden(msg.into())
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Database(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn public_message(&self) -> String {
        match self {
            Self::NotFound(m)
            | Self::BadRequest(m)
            | Self::Unauthorized(m)
            | Self::Forbidden(m)
            | Self::Conflict(m)
            | Self::Validation(m)
            | Self::RateLimited(m) => m.clone(),
            Self::Database(e) => {
                tracing::error!(error = %e, "database error");
                "Something went wrong on our side.".to_string()
            }
            Self::Internal(m) => {
                tracing::error!(error = %m, "internal error");
                "Something went wrong on our side.".to_string()
            }
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.public_message())
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        // Unique-constraint violations are expected during normal editing.
        if let sqlx::Error::Database(db) = &e {
            if db.is_unique_violation() {
                return Self::Conflict("That value is already taken.".into());
            }
        }
        Self::Database(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

impl From<askama::Error> for AppError {
    fn from(e: askama::Error) -> Self {
        Self::Internal(format!("template error: {e}"))
    }
}

// Errors that should be surfaced to the user in a friendly way.
pub trait FlashMessage {
    fn flash(&self) -> Option<(String, String)>;
}

impl FlashMessage for AppError {
    fn flash(&self) -> Option<(String, String)> {
        match self {
            Self::NotFound(m)
            | Self::BadRequest(m)
            | Self::Unauthorized(m)
            | Self::Forbidden(m)
            | Self::Conflict(m)
            | Self::Validation(m)
            | Self::RateLimited(m) => Some(("error".into(), m.clone())),
            _ => None,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let message = self.public_message();

        if wants_json() {
            let payload = serde_json::json!({
                "error": status.canonical_reason().unwrap_or("Error"),
                "status": status.as_u16(),
                "message": message,
            });
            return (status, Json(payload)).into_response();
        }

        let body = crate::templates::error_page(status, &message);
        let mut res = (status, axum::body::Body::from(body)).into_response();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/html; charset=utf-8"),
        );
        res
    }
}

// Detect if the caller asked for JSON. Uses a thread-local flag set by the
// `/api` router so that the error type stays decoupled from request parts.
thread_local! {
    static JSON_CONTEXT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn set_json_context(value: bool) {
    JSON_CONTEXT.with(|c| c.set(value));
}

fn wants_json() -> bool {
    JSON_CONTEXT.with(|c| c.get())
}

/// A small helper used by handlers that want to redirect back with a flash
/// message stored in the session.
pub fn redirect_with_flash(redirect: &str, kind: &str, message: &str) -> Response {
    let encoded = format!("{kind}:{message}");
    let target = if redirect.contains('?') {
        format!("{redirect}&flash={}", urlencode(&encoded))
    } else {
        format!("{redirect}?flash={}", urlencode(&encoded))
    };
    axum::response::Redirect::to(&target).into_response()
}

/// Percent-encode a string for use in a query string.
pub fn urlencode(input: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(input, NON_ALPHANUMERIC).to_string()
}
