//! Admin: social links management.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect};
use crate::state::AppState;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/socials.html")]
pub struct SocialsTemplate {
    pub view: AdminSocialsView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) =
        admin_context(&state, &auth.0, &headers, "/admin/socials", "socials").await?;

    let rows: Vec<crate::models::db::SocialLink> =
        sqlx::query_as("SELECT * FROM social_links ORDER BY sort_order ASC, id ASC")
            .fetch_all(&state.db)
            .await?;
    let socials = rows
        .iter()
        .map(|s| AdminSocialRow {
            id: s.id,
            platform: s.platform.clone(),
            url: s.url.clone(),
            label: s.label.clone().unwrap_or_default(),
            sort_order: s.sort_order,
        })
        .collect();

    let view = AdminSocialsView {
        ctx,
        base,
        csrf_token: auth.0.csrf_token.clone(),
        socials,
    };
    Ok(crate::templates::Html(SocialsTemplate { view }).into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct SocialForm {
    pub id: Option<i64>,
    pub platform: String,
    pub url: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub csrf_token: String,
}

pub async fn store(
    State(state): State<AppState>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<SocialForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let platform = form.platform.trim().to_string();
    let url = form.url.trim().to_string();
    if platform.is_empty() || url.is_empty() {
        return Err(AppError::validation("Platform and URL are required."));
    }

    match form.id {
        Some(id) => {
            sqlx::query(
                "UPDATE social_links SET platform = ?, url = ?, label = ?, sort_order = ? WHERE id = ?",
            )
            .bind(&platform)
            .bind(&url)
            .bind(non_empty(&form.label))
            .bind(form.sort_order)
            .bind(id)
            .execute(&state.db)
            .await?;
        }
        None => {
            sqlx::query(
                "INSERT INTO social_links (platform, url, label, sort_order) VALUES (?, ?, ?, ?)",
            )
            .bind(&platform)
            .bind(&url)
            .bind(non_empty(&form.label))
            .bind(form.sort_order)
            .execute(&state.db)
            .await?;
        }
    }

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "social.update",
        Some("social"),
        None,
        Some(&platform),
    )
    .await;

    Ok(flash_redirect("/admin/socials", "success", "Saved."))
}

pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CsrfOnly>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;
    sqlx::query("DELETE FROM social_links WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(flash_redirect("/admin/socials", "success", "Deleted."))
}

#[derive(Debug, serde::Deserialize)]
pub struct CsrfOnly {
    pub csrf_token: String,
}

fn non_empty(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}
