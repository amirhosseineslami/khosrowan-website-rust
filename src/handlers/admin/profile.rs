//! Admin: the current user's own profile.

use askama::Template;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::auth::password::{self, strength_label, strength_score};
use crate::auth::session;
use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect};
use crate::state::AppState;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/profile.html")]
pub struct ProfileTemplate {
    pub view: AdminProfileView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(&state, &auth.0, &headers, "/admin/profile", "profile").await?;

    let row: Option<crate::models::db::User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(auth.0.user_id)
        .fetch_optional(&state.db)
        .await?;

    let Some(row) = row else {
        return Err(AppError::not_found("Your account no longer exists."));
    };

    let view = AdminProfileView {
        ctx,
        base,
        csrf_token: auth.0.csrf_token.clone(),
        error: String::new(),
        success: false,
        username: row.username,
        email: row.email,
        display_name: row.display_name,
        bio: row.bio.unwrap_or_default(),
        avatar: row.avatar.unwrap_or_default(),
        password_strength: 0,
        password_strength_label: strength_label(0).to_string(),
    };

    Ok(crate::templates::Html(ProfileTemplate { view }).into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct ProfileForm {
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub bio: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub current_password: String,
    #[serde(default)]
    pub new_password: String,
    #[serde(default)]
    pub new_password_confirmation: String,
    #[serde(default)]
    pub csrf_token: String,
}

pub async fn update(
    State(state): State<AppState>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<ProfileForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let email = form.email.trim().to_lowercase();
    if !crate::util::is_valid_email(&email) {
        return Err(AppError::validation(
            "Please provide a valid e-mail address.",
        ));
    }

    // Changing the password requires the current one.
    if !form.new_password.is_empty() {
        let hash: Option<String> =
            sqlx::query_scalar("SELECT password_hash FROM users WHERE id = ?")
                .bind(auth.0.user_id)
                .fetch_optional(&state.db)
                .await?;

        let Some(hash) = hash else {
            return Err(AppError::not_found("Your account no longer exists."));
        };

        if !password::verify_password(&form.current_password, &hash) {
            return Err(AppError::validation(
                "Your current password is not correct.",
            ));
        }
        if form.new_password.len() < 10 {
            return Err(AppError::validation(
                "The new password must be at least 10 characters long.",
            ));
        }
        if form.new_password != form.new_password_confirmation {
            return Err(AppError::validation("The two passwords do not match."));
        }
        if strength_score(&form.new_password) < 2 {
            return Err(AppError::validation(
                "That password is too weak — mix upper case, lower case, digits and symbols.",
            ));
        }

        let new_hash = password::hash_password(&form.new_password)
            .map_err(|e| AppError::internal(format!("could not hash password: {e}")))?;

        sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
            .bind(&new_hash)
            .bind(chrono::Utc::now())
            .bind(auth.0.user_id)
            .execute(&state.db)
            .await?;

        // Force a re-login everywhere else.
        sqlx::query("DELETE FROM sessions WHERE user_id = ? AND id != ?")
            .bind(auth.0.user_id)
            .bind(&auth.0.session_id)
            .execute(&state.db)
            .await?;
    }

    sqlx::query(
        "UPDATE users SET display_name = ?, email = ?, bio = ?, avatar = ?, updated_at = ? WHERE id = ?",
    )
    .bind(form.display_name.trim())
    .bind(&email)
    .bind(non_empty(&form.bio))
    .bind(non_empty(&form.avatar))
    .bind(chrono::Utc::now())
    .bind(auth.0.user_id)
    .execute(&state.db)
    .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "profile.update",
        Some("user"),
        None,
        None,
    )
    .await;

    let _ = session::touch_session(&state.db, &auth.0.session_id).await;

    Ok(flash_redirect(
        "/admin/profile",
        "success",
        "Profile updated.",
    ))
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
