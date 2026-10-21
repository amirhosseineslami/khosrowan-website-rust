//! Custom extractors for authentication and CSRF.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::auth::session::AuthContext;
use crate::error::AppError;
use crate::state::AppState;

/// Optional authentication context.
#[derive(Debug, Clone)]
pub struct MaybeAuth(pub Option<AuthContext>);

impl MaybeAuth {
    pub fn is_authenticated(&self) -> bool {
        self.0.is_some()
    }
    pub fn user_id(&self) -> Option<i64> {
        self.0.as_ref().map(|a| a.user_id)
    }
    pub fn display_name(&self) -> Option<&str> {
        self.0.as_ref().map(|a| a.display_name.as_str())
    }
    pub fn is_owner(&self) -> bool {
        self.0.as_ref().map(|a| a.is_owner()).unwrap_or(false)
    }
}

impl FromRequestParts<AppState> for MaybeAuth {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(MaybeAuth(parts.extensions.get::<AuthContext>().cloned()))
    }
}

/// Requires an authenticated administrator.
#[derive(Debug, Clone)]
pub struct RequireAdmin(pub AuthContext);

impl FromRequestParts<AppState> for RequireAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth = parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .ok_or_else(|| AppError::unauthorized("You need to sign in to continue."))?;

        if !auth.is_admin() {
            return Err(AppError::forbidden(
                "Your account does not have access to the admin panel.",
            ));
        }
        Ok(RequireAdmin(auth))
    }
}

/// Requires the `owner` role.
#[derive(Debug, Clone)]
pub struct RequireOwner(pub AuthContext);

impl FromRequestParts<AppState> for RequireOwner {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth = parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .ok_or_else(|| AppError::unauthorized("You need to sign in to continue."))?;

        if !auth.is_owner() {
            return Err(AppError::forbidden(
                "Only the site owner can perform this action.",
            ));
        }
        Ok(RequireOwner(auth))
    }
}

/// Validate a CSRF token against the session.
pub fn verify_csrf(auth: &AuthContext, provided: &str) -> Result<(), AppError> {
    if provided.is_empty() {
        return Err(AppError::forbidden(
            "Missing CSRF token. Please reload the page and try again.",
        ));
    }
    if crate::auth::session::verify_csrf(auth, provided) {
        Ok(())
    } else {
        Err(AppError::forbidden(
            "Invalid CSRF token. Please reload the page and try again.",
        ))
    }
}
