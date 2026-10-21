//! Admin: user management.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};

use crate::auth::password::{self, strength_label, strength_score};
use crate::auth::{session, totp};
use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect, status_class};
use crate::state::AppState;
use crate::util;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/users.html")]
pub struct UsersTemplate {
    pub view: AdminUsersView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(&state, &auth.0, &headers, "/admin/users", "users").await?;

    let rows: Vec<crate::models::db::User> = sqlx::query_as("SELECT * FROM users ORDER BY id ASC")
        .fetch_all(&state.db)
        .await?;

    let users = rows
        .iter()
        .map(|u| AdminUserRow {
            id: u.id,
            username: u.username.clone(),
            display_name: u.display_name.clone(),
            email: u.email.clone(),
            role: u.role.clone(),
            role_class: status_class(&u.role).to_string(),
            is_active: u.is_active,
            totp_enabled: u.totp_enabled,
            last_login_at: util::time_ago(u.last_login_at),
            created_at: util::format_date(u.created_at),
        })
        .collect();

    let session_rows: Vec<crate::models::db::Session> =
        sqlx::query_as("SELECT * FROM sessions ORDER BY last_seen_at DESC LIMIT 50")
            .fetch_all(&state.db)
            .await?;

    let mut sessions: Vec<AdminSessionRow> = Vec::new();
    for s in &session_rows {
        let username: Option<String> =
            sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
                .bind(s.user_id)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten();

        sessions.push(AdminSessionRow {
            id: s.id.clone(),
            username: username.unwrap_or_default(),
            ip: s.ip.clone().unwrap_or_default(),
            user_agent: s.user_agent.clone().unwrap_or_default(),
            created_at: util::format_datetime(s.created_at),
            expires_at: util::format_datetime(s.expires_at),
            is_current: s.id == auth.0.session_id,
        });
    }

    let view = AdminUsersView {
        ctx,
        base,
        users,
        sessions,
    };
    Ok(crate::templates::Html(UsersTemplate { view }).into_response())
}

#[derive(Template)]
#[template(path = "admin/user_form.html")]
pub struct UserFormTemplate {
    pub view: AdminUserFormView,
}

pub async fn create(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireOwner,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(&state, &auth.0, &headers, "/admin/users/new", "users").await?;
    let view = AdminUserFormView {
        ctx,
        base,
        user: None,
        is_new: true,
        csrf_token: auth.0.csrf_token.clone(),
        error: String::new(),
        totp_secret: String::new(),
        totp_uri: String::new(),
        recovery_codes: Vec::new(),
    };
    Ok(crate::templates::Html(UserFormTemplate { view }).into_response())
}

pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireOwner,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(
        &state,
        &auth.0,
        &headers,
        &format!("/admin/users/{id}/edit"),
        "users",
    )
    .await?;

    let row: Option<crate::models::db::User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;

    let Some(row) = row else {
        return Err(AppError::not_found("That user does not exist."));
    };

    let user = AdminUserEdit {
        id: row.id,
        username: row.username.clone(),
        email: row.email.clone(),
        display_name: row.display_name.clone(),
        role: row.role.clone(),
        bio: row.bio.clone().unwrap_or_default(),
        avatar: row.avatar.clone().unwrap_or_default(),
        is_active: row.is_active,
        totp_enabled: row.totp_enabled,
        last_login_at: util::format_datetime(row.last_login_at),
        created_at: util::format_date(row.created_at),
    };

    let view = AdminUserFormView {
        ctx,
        base,
        user: Some(user),
        is_new: false,
        csrf_token: auth.0.csrf_token.clone(),
        error: String::new(),
        totp_secret: String::new(),
        totp_uri: String::new(),
        recovery_codes: Vec::new(),
    };
    Ok(crate::templates::Html(UserFormTemplate { view }).into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct UserForm {
    pub username: String,
    pub email: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub bio: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub is_active: Option<String>,
    #[serde(default)]
    pub enable_totp: Option<String>,
    #[serde(default)]
    pub csrf_token: String,
}

pub async fn store(
    State(state): State<AppState>,
    auth: crate::extract::RequireOwner,
    axum::extract::Form(form): axum::extract::Form<UserForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let username = form.username.trim().to_lowercase();
    let email = form.email.trim().to_lowercase();

    if username.is_empty() || username.len() > 60 {
        return Err(AppError::validation("Please choose a valid username."));
    }
    if !util::is_valid_email(&email) {
        return Err(AppError::validation(
            "Please choose a valid e-mail address.",
        ));
    }
    if form.password.len() < 10 {
        return Err(AppError::validation(
            "The password must be at least 10 characters long.",
        ));
    }

    let hash = password::hash_password(&form.password)
        .map_err(|e| AppError::internal(format!("could not hash password: {e}")))?;

    let role = if form.role == "owner" {
        "owner"
    } else {
        "editor"
    };
    let totp_secret = if form.enable_totp.is_some() {
        Some(totp::generate_secret())
    } else {
        None
    };

    sqlx::query(
        "INSERT INTO users (username, email, password_hash, role, display_name, bio, avatar, is_active, totp_secret, totp_enabled)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&username)
    .bind(&email)
    .bind(&hash)
    .bind(role)
    .bind(form.display_name.trim())
    .bind(non_empty(&form.bio))
    .bind(non_empty(&form.avatar))
    .bind(form.is_active.is_some() || role == "owner")
    .bind(&totp_secret)
    .bind(totp_secret.is_some())
    .execute(&state.db)
    .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "user.create",
        Some("user"),
        None,
        Some(&username),
    )
    .await;

    Ok(flash_redirect("/admin/users", "success", "User created."))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireOwner,
    axum::extract::Form(form): axum::extract::Form<UserForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let username = form.username.trim().to_lowercase();
    let email = form.email.trim().to_lowercase();
    if username.is_empty() || !util::is_valid_email(&email) {
        return Err(AppError::validation(
            "Please provide a valid username and e-mail address.",
        ));
    }

    let role = if form.role == "owner" {
        "owner"
    } else {
        "editor"
    };
    let is_active = form.is_active.is_some();

    // Never allow the last owner to be demoted or deactivated.
    let owners: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'owner' AND is_active = 1")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    let current_role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    if current_role.as_deref() == Some("owner") && (role != "owner" || !is_active) && owners <= 1 {
        return Err(AppError::validation(
            "This is the last owner account — promote another user first.",
        ));
    }

    sqlx::query(
        "UPDATE users SET username = ?, email = ?, display_name = ?, role = ?, bio = ?, avatar = ?, is_active = ?
         WHERE id = ?",
    )
    .bind(&username)
    .bind(&email)
    .bind(form.display_name.trim())
    .bind(role)
    .bind(non_empty(&form.bio))
    .bind(non_empty(&form.avatar))
    .bind(is_active)
    .bind(id)
    .execute(&state.db)
    .await?;

    if !form.password.is_empty() {
        if form.password.len() < 10 {
            return Err(AppError::validation(
                "The password must be at least 10 characters long.",
            ));
        }
        let hash = password::hash_password(&form.password)
            .map_err(|e| AppError::internal(format!("could not hash password: {e}")))?;
        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(&hash)
            .bind(id)
            .execute(&state.db)
            .await?;
        session::destroy_user_sessions(&state.db, id).await;
    }

    if form.enable_totp.is_some() {
        let secret = totp::generate_secret();
        sqlx::query("UPDATE users SET totp_secret = ?, totp_enabled = 1 WHERE id = ?")
            .bind(&secret)
            .bind(id)
            .execute(&state.db)
            .await?;
    } else {
        sqlx::query("UPDATE users SET totp_enabled = 0, totp_secret = NULL WHERE id = ?")
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "user.update",
        Some("user"),
        Some(&id.to_string()),
        Some(&username),
    )
    .await;

    Ok(flash_redirect("/admin/users", "success", "User updated."))
}

pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireOwner,
    axum::extract::Form(form): axum::extract::Form<CsrfOnly>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    if id == auth.0.user_id {
        return Err(AppError::validation("You cannot delete your own account."));
    }

    let owners: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'owner' AND is_active = 1")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    if role.as_deref() == Some("owner") && owners <= 1 {
        return Err(AppError::validation(
            "This is the last owner account and cannot be deleted.",
        ));
    }

    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "user.delete",
        Some("user"),
        Some(&id.to_string()),
        None,
    )
    .await;

    Ok(flash_redirect("/admin/users", "success", "User deleted."))
}

pub async fn revoke_sessions(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireOwner,
    axum::extract::Form(form): axum::extract::Form<CsrfOnly>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    session::destroy_user_sessions(&state.db, id).await;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "user.revoke_sessions",
        Some("user"),
        Some(&id.to_string()),
        None,
    )
    .await;

    Ok(flash_redirect(
        "/admin/users",
        "success",
        "Sessions revoked.",
    ))
}

/// Endpoint that generates a fresh 2FA secret and recovery codes.
pub async fn generate_totp(
    State(state): State<AppState>,
    auth: crate::extract::RequireOwner,
    axum::extract::Form(form): axum::extract::Form<CsrfOnly>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let secret = totp::generate_secret();
    let uri = totp::provisioning_uri(&secret, &auth.0.email, &state.settings.snapshot().title);
    let codes = totp::recovery_codes(8);

    sqlx::query("UPDATE users SET totp_secret = ?, totp_enabled = 0 WHERE id = ?")
        .bind(&secret)
        .bind(auth.0.user_id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "user.totp_setup",
        Some("user"),
        None,
        None,
    )
    .await;

    Ok(serde_json::to_string(&serde_json::json!({
        "secret": secret,
        "uri": uri,
        "recovery_codes": codes,
    }))
    .unwrap_or_default()
    .into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct CsrfOnly {
    pub csrf_token: String,
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Re-exported for the profile page.
pub fn password_strength_label(password: &str) -> &'static str {
    strength_label(strength_score(password))
}
