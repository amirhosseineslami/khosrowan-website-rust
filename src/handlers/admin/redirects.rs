//! Admin: URL redirects.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect};
use crate::state::AppState;
use crate::util;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/redirects.html")]
pub struct RedirectsTemplate {
    pub view: AdminRedirectsView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) =
        admin_context(&state, &auth.0, &headers, "/admin/redirects", "redirects").await?;

    let rows: Vec<crate::models::db::Redirect> =
        sqlx::query_as("SELECT * FROM redirects ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?;

    let redirects = rows
        .iter()
        .map(|r| AdminRedirectRow {
            id: r.id,
            from_path: r.from_path.clone(),
            to_path: r.to_path.clone(),
            status_code: r.status_code,
            hits: r.hits,
            is_active: r.is_active,
            created_at: util::format_date(r.created_at),
        })
        .collect();

    let view = AdminRedirectsView {
        ctx,
        base,
        redirects,
        csrf_token: auth.0.csrf_token.clone(),
    };

    Ok(crate::templates::Html(RedirectsTemplate { view }).into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct RedirectForm {
    pub from_path: String,
    pub to_path: String,
    #[serde(default)]
    pub status_code: String,
    #[serde(default)]
    pub csrf_token: String,
}

pub async fn store(
    State(state): State<AppState>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<RedirectForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let from = form.from_path.trim().to_string();
    let to = form.to_path.trim().to_string();

    if !from.starts_with('/') {
        return Err(AppError::validation(
            "The source path must start with a slash.",
        ));
    }
    if to.is_empty() {
        return Err(AppError::validation("Please provide a destination."));
    }

    let status_code = form.status_code.trim().parse::<i64>().unwrap_or(301);
    let status_code = if [301, 302, 307, 308].contains(&status_code) {
        status_code
    } else {
        301
    };

    sqlx::query(
        "INSERT INTO redirects (from_path, to_path, status_code, is_active)
         VALUES (?, ?, ?, 1)
         ON CONFLICT(from_path) DO UPDATE SET to_path = excluded.to_path,
             status_code = excluded.status_code, is_active = 1",
    )
    .bind(&from)
    .bind(&to)
    .bind(status_code)
    .execute(&state.db)
    .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "redirect.create",
        Some("redirect"),
        None,
        Some(&from),
    )
    .await;

    Ok(flash_redirect(
        "/admin/redirects",
        "success",
        "Redirect saved.",
    ))
}

pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CsrfOnly>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    sqlx::query("DELETE FROM redirects WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "redirect.delete",
        Some("redirect"),
        Some(&id.to_string()),
        None,
    )
    .await;

    Ok(flash_redirect(
        "/admin/redirects",
        "success",
        "Redirect deleted.",
    ))
}

#[derive(Debug, serde::Deserialize)]
pub struct CsrfOnly {
    pub csrf_token: String,
}
