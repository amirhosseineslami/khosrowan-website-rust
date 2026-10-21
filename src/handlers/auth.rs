//! Authentication endpoints: login, logout, two-factor and password reset.

use askama::Template;
use axum::extract::{Form, Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::auth::session;
use crate::auth::{password, totp};
use crate::error::{AppError, AppResult};
use crate::handlers::context;
use crate::state::AppState;
use crate::util;
use crate::views::AuthView;

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "admin/login.html")]
pub struct LoginTemplate {
    pub view: AuthView,
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub remember: Option<String>,
    #[serde(default)]
    pub next: Option<String>,
    /// Set when the second step (TOTP) is being submitted.
    #[serde(default)]
    pub stage: Option<String>,
}

pub async fn login_form(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<LoginParams>,
) -> AppResult<Response> {
    let base = context::build(&state, None, &headers, "/admin/login").await?;
    let view = AuthView {
        base,
        error: String::new(),
        username: String::new(),
        next: params.next.unwrap_or_default(),
        totp_required: false,
        show_2fa_hint: false,
    };
    Ok(crate::templates::Html(LoginTemplate { view }).into_response())
}

#[derive(Debug, Deserialize)]
pub struct LoginParams {
    pub next: Option<String>,
}

pub async fn login_submit(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(form): Form<LoginForm>,
) -> AppResult<Response> {
    let next = form.next.clone().unwrap_or_else(|| "/admin".into());

    // Step two: the user already passed the password check and now supplies a
    // one-time code.
    if form.stage.as_deref() == Some("totp") {
        return complete_totp_login(&state, &headers, &form, &next).await;
    }

    let username = form.username.trim().to_lowercase();
    let user: Option<crate::models::db::User> =
        sqlx::query_as("SELECT * FROM users WHERE username = ? OR lower(email) = ?")
            .bind(&username)
            .bind(&username)
            .fetch_optional(&state.db)
            .await?;

    let Some(user) = user else {
        // Do not leak whether the account exists.
        tracing::warn!(username = %username, "failed login: unknown user");
        return Ok(render_login_error(
            &state,
            &headers,
            "Those credentials do not match our records.".into(),
            &username,
            &next,
            false,
        )
        .await?);
    };

    if !user.is_active {
        return Ok(render_login_error(
            &state,
            &headers,
            "This account has been deactivated.".into(),
            &username,
            &next,
            false,
        )
        .await?);
    }

    // Respect a temporary lockout.
    if let Some(locked_until) = user.locked_until {
        if locked_until > chrono::Utc::now() {
            let minutes = (locked_until - chrono::Utc::now()).num_minutes() + 1;
            return Ok(render_login_error(
                &state,
                &headers,
                format!("Too many failed attempts. Try again in {minutes} minute(s)."),
                &username,
                &next,
                false,
            )
            .await?);
        }
    }

    if !password::verify_password(&form.password, &user.password_hash) {
        let failed = user.failed_logins + 1;
        let locked_until = if failed >= crate::auth::MAX_FAILED_LOGINS {
            Some(chrono::Utc::now() + chrono::Duration::minutes(crate::auth::LOCKOUT_MINUTES))
        } else {
            None
        };
        let _ = sqlx::query("UPDATE users SET failed_logins = ?, locked_until = ? WHERE id = ?")
            .bind(failed)
            .bind(locked_until)
            .bind(user.id)
            .execute(&state.db)
            .await;

        tracing::warn!(username = %user.username, "failed login: bad password");
        return Ok(render_login_error(
            &state,
            &headers,
            "Those credentials do not match our records.".into(),
            &username,
            &next,
            false,
        )
        .await?);
    }

    if user.totp_enabled && user.totp_secret.is_some() {
        // Ask for the one-time code.
        let base = context::build(&state, None, &headers, "/admin/login").await?;
        let view = AuthView {
            base,
            error: String::new(),
            username: user.username.clone(),
            next: next.clone(),
            totp_required: true,
            show_2fa_hint: true,
        };
        return Ok(crate::templates::Html(LoginTemplate { view }).into_response());
    }

    finish_login(&state, &headers, &user, &next, form.remember.is_some()).await
}

async fn complete_totp_login(
    state: &AppState,
    headers: &axum::http::HeaderMap,
    form: &LoginForm,
    next: &str,
) -> AppResult<Response> {
    let username = form.username.trim().to_lowercase();
    let user: Option<crate::models::db::User> =
        sqlx::query_as("SELECT * FROM users WHERE username = ?")
            .bind(&username)
            .fetch_optional(&state.db)
            .await?;

    let Some(user) = user else {
        return Ok(render_login_error(
            state,
            headers,
            "That verification code was not accepted.".into(),
            &username,
            next,
            true,
        )
        .await?);
    };

    let valid = match &user.totp_secret {
        Some(secret) => totp::verify(secret, &form.code, chrono::Utc::now().timestamp()),
        None => false,
    };

    if !valid {
        tracing::warn!(username = %user.username, "failed login: invalid totp");
        return Ok(render_login_error(
            state,
            headers,
            "That verification code was not accepted.".into(),
            &username,
            next,
            true,
        )
        .await?);
    }

    finish_login(state, headers, &user, next, form.remember.is_some()).await
}

async fn finish_login(
    state: &AppState,
    headers: &axum::http::HeaderMap,
    user: &crate::models::db::User,
    next: &str,
    remember: bool,
) -> AppResult<Response> {
    let ip = util::client_ip(headers);
    let agent = util::user_agent(headers);

    let (session_id, _csrf, _expires) = session::create_session(
        &state.db,
        &state.config,
        user.id,
        ip.as_deref(),
        agent.as_deref(),
        remember,
    )
    .await?;

    sqlx::query(
        "UPDATE users SET failed_logins = 0, locked_until = NULL, last_login_at = ? WHERE id = ?",
    )
    .bind(chrono::Utc::now())
    .bind(user.id)
    .execute(&state.db)
    .await?;

    crate::audit(
        state,
        Some(user.id),
        "auth.login",
        Some("user"),
        Some(&user.id.to_string()),
        None,
    )
    .await;

    let max_age = if remember {
        state.config.remember_ttl.as_secs() as i64
    } else {
        state.config.session_ttl.as_secs() as i64
    };

    let cookie = session::session_cookie(&session_id, max_age, state.use_secure_cookies());

    // Only allow relative redirect targets.
    let target = if next.starts_with('/') && !next.starts_with("//") {
        next.to_string()
    } else {
        "/admin".to_string()
    };

    Ok((
        [(axum::http::header::SET_COOKIE, cookie.to_string())],
        Redirect::to(&target),
    )
        .into_response())
}

async fn render_login_error(
    state: &AppState,
    headers: &axum::http::HeaderMap,
    error: String,
    username: &str,
    next: &str,
    totp_required: bool,
) -> AppResult<Response> {
    let base = context::build(state, None, headers, "/admin/login").await?;
    let view = AuthView {
        base,
        error,
        username: username.to_string(),
        next: next.to_string(),
        totp_required,
        show_2fa_hint: totp_required,
    };
    Ok(crate::templates::Html(LoginTemplate { view }).into_response())
}

// ---------------------------------------------------------------------------
// Logout
// ---------------------------------------------------------------------------

pub async fn logout(
    State(state): State<AppState>,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    if let Some(auth) = &auth.0 {
        session::destroy_session(&state.db, &auth.session_id).await;
        crate::audit(
            &state,
            Some(auth.user_id),
            "auth.logout",
            Some("user"),
            None,
            None,
        )
        .await;
    }

    let cookie = session::clear_session_cookie(state.use_secure_cookies());
    Ok((
        [(axum::http::header::SET_COOKIE, cookie.to_string())],
        Redirect::to("/admin/login"),
    )
        .into_response())
}

// ---------------------------------------------------------------------------
// Password reset
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "admin/reset_password.html")]
pub struct ResetPasswordTemplate {
    pub view: ResetView,
}

#[derive(Debug, Clone)]
pub struct ResetView {
    pub base: crate::views::BaseContext,
    pub token: String,
    pub username: String,
    pub error: String,
    pub success: bool,
    pub message: String,
}

#[derive(Template)]
#[template(path = "admin/forgot_password.html")]
pub struct ForgotPasswordTemplate {
    pub view: ForgotView,
}

#[derive(Debug, Clone)]
pub struct ForgotView {
    pub base: crate::views::BaseContext,
    pub error: String,
    pub success: bool,
    pub message: String,
    pub email: String,
}

pub async fn forgot_form(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> AppResult<Response> {
    let base = context::build(&state, None, &headers, "/admin/forgot-password").await?;
    let view = ForgotView {
        base,
        error: String::new(),
        success: false,
        message: String::new(),
        email: String::new(),
    };
    Ok(crate::templates::Html(ForgotPasswordTemplate { view }).into_response())
}

#[derive(Debug, Deserialize)]
pub struct ForgotForm {
    pub email: String,
}

pub async fn forgot_submit(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(form): Form<ForgotForm>,
) -> AppResult<Response> {
    let base = context::build(&state, None, &headers, "/admin/forgot-password").await?;
    let email = form.email.trim().to_lowercase();

    let mut error = String::new();
    let mut message = String::new();
    let mut success = false;

    if !util::is_valid_email(&email) {
        error = "Please enter a valid e-mail address.".into();
    } else {
        let user: Option<i64> =
            sqlx::query_scalar("SELECT id FROM users WHERE lower(email) = ? AND is_active = 1")
                .bind(&email)
                .fetch_optional(&state.db)
                .await?;

        if let Some(user_id) = user {
            let token = uuid::Uuid::new_v4().to_string();
            let expires = chrono::Utc::now() + chrono::Duration::hours(2);

            sqlx::query(
                "INSERT INTO password_resets (user_id, token, expires_at) VALUES (?, ?, ?)",
            )
            .bind(user_id)
            .bind(&token)
            .bind(expires)
            .execute(&state.db)
            .await?;

            let link = format!(
                "{}/admin/reset-password/{}",
                state.config.base_url.trim_end_matches('/'),
                token
            );

            let html = crate::mail::html_wrapper(
                "Reset your password",
                &format!(
                    "<p>We received a request to reset your password.</p><p><a href=\"{link}\" style=\"display:inline-block;padding:12px 20px;background:#6366f1;color:#0b0d12;border-radius:10px;text-decoration:none;font-weight:600;\">Choose a new password</a></p><p style=\"color:#6b7280;font-size:13px;\">This link expires in two hours. If you did not request this, you can ignore this email.</p>"
                ),
                &state.settings.snapshot().theme_accent,
            );

            if let Err(err) = state
                .mailer
                .send(&email, "Reset your password", &link, Some(&html))
                .await
            {
                tracing::warn!(error = %err, "failed to send password reset email");
            }
        }

        // Always show the same message to avoid account enumeration.
        success = true;
        message = "If an account exists for that address, a reset link is on its way.".into();
    }

    let view = ForgotView {
        base,
        error,
        success,
        message,
        email,
    };
    Ok(crate::templates::Html(ForgotPasswordTemplate { view }).into_response())
}

#[derive(Debug, Deserialize)]
pub struct ResetQuery {
    pub token: String,
}

pub async fn reset_form(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(params): Query<ResetQuery>,
) -> AppResult<Response> {
    let base = context::build(&state, None, &headers, "/admin/reset-password").await?;

    let valid: Option<String> = sqlx::query_scalar(
        "SELECT username FROM password_resets WHERE token = ? AND expires_at > ? AND used_at IS NULL",
    )
    .bind(&params.token)
    .bind(chrono::Utc::now())
    .fetch_optional(&state.db)
    .await?;

    let view = ResetView {
        base,
        token: params.token.clone(),
        username: valid.clone().unwrap_or_default(),
        error: if valid.is_none() {
            "This reset link is invalid or has expired.".into()
        } else {
            String::new()
        },
        success: false,
        message: String::new(),
    };

    Ok(crate::templates::Html(ResetPasswordTemplate { view }).into_response())
}

#[derive(Debug, Deserialize)]
pub struct ResetForm {
    pub token: String,
    pub password: String,
    pub password_confirmation: String,
}

pub async fn reset_submit(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(form): Form<ResetForm>,
) -> AppResult<Response> {
    let base = context::build(&state, None, &headers, "/admin/reset-password").await?;

    let mut error = String::new();
    let username;

    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT user_id, username FROM password_resets WHERE token = ? AND expires_at > ? AND used_at IS NULL",
    )
    .bind(&form.token)
    .bind(chrono::Utc::now())
    .fetch_optional(&state.db)
    .await?;

    let Some((user_id, user_name)) = row else {
        error = "This reset link is invalid or has expired.".into();
        username = String::new();
        let view = ResetView {
            base,
            token: form.token,
            username,
            error,
            success: false,
            message: String::new(),
        };
        return Ok(crate::templates::Html(ResetPasswordTemplate { view }).into_response());
    };
    username = user_name;

    if form.password.len() < 10 {
        error = "Please choose a password of at least 10 characters.".into();
    } else if form.password != form.password_confirmation {
        error = "The two passwords do not match.".into();
    } else if password::strength_score(&form.password) < 2 {
        error =
            "That password is too weak — mix upper case, lower case, digits and symbols.".into();
    }

    if !error.is_empty() {
        let view = ResetView {
            base,
            token: form.token,
            username,
            error,
            success: false,
            message: String::new(),
        };
        return Ok(crate::templates::Html(ResetPasswordTemplate { view }).into_response());
    }

    let hash = password::hash_password(&form.password)
        .map_err(|e| AppError::internal(format!("could not hash password: {e}")))?;

    sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
        .bind(&hash)
        .bind(chrono::Utc::now())
        .bind(user_id)
        .execute(&state.db)
        .await?;

    sqlx::query("UPDATE password_resets SET used_at = ? WHERE token = ?")
        .bind(chrono::Utc::now())
        .bind(&form.token)
        .execute(&state.db)
        .await?;

    // Force every existing session to be invalidated.
    session::destroy_user_sessions(&state.db, user_id).await;

    crate::audit(
        &state,
        Some(user_id),
        "auth.password_reset",
        Some("user"),
        None,
        None,
    )
    .await;

    let view = ResetView {
        base,
        token: String::new(),
        username,
        error: String::new(),
        success: true,
        message: "Your password has been changed. You can now sign in.".into(),
    };
    Ok(crate::templates::Html(ResetPasswordTemplate { view }).into_response())
}
