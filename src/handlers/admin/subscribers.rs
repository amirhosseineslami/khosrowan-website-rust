//! Admin: newsletter subscribers.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};

use crate::error::AppResult;
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect, status_class};
use crate::state::AppState;
use crate::util;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/subscribers.html")]
pub struct SubscribersTemplate {
    pub view: AdminSubscribersView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(
        &state,
        &auth.0,
        &headers,
        "/admin/subscribers",
        "subscribers",
    )
    .await?;

    let rows: Vec<crate::models::db::Subscriber> =
        sqlx::query_as("SELECT * FROM subscribers ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?;

    let subscribers = rows
        .iter()
        .map(|s| AdminSubscriberRow {
            id: s.id,
            email: s.email.clone(),
            name: s.name.clone().unwrap_or_default(),
            status: s.status.clone(),
            status_class: status_class(&s.status).to_string(),
            source: s.source.clone().unwrap_or_default(),
            created_at: util::format_date(s.created_at),
            token: s.token.clone(),
        })
        .collect();

    let counts: Vec<ChartPoint> = sqlx::query_as::<_, (String, i64)>(
        "SELECT status, COUNT(*) FROM subscribers GROUP BY status ORDER BY COUNT(*) DESC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(label, value)| ChartPoint { label, value })
    .collect();

    let view = AdminSubscribersView {
        ctx,
        base,
        subscribers,
        counts,
    };

    Ok(crate::templates::Html(SubscribersTemplate { view }).into_response())
}

pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CsrfOnly>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    sqlx::query("DELETE FROM subscribers WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "subscriber.delete",
        Some("subscriber"),
        Some(&id.to_string()),
        None,
    )
    .await;

    Ok(flash_redirect(
        "/admin/subscribers",
        "success",
        "Subscriber removed.",
    ))
}

/// Export the subscriber list as CSV.
pub async fn export(
    State(state): State<AppState>,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let rows: Vec<crate::models::db::Subscriber> =
        sqlx::query_as("SELECT * FROM subscribers ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?;

    let mut csv = String::from("email,name,status,source,subscribed_at\n");
    for row in rows {
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            escape_csv(&row.email),
            escape_csv(row.name.as_deref().unwrap_or("")),
            escape_csv(&row.status),
            escape_csv(row.source.as_deref().unwrap_or("")),
            util::format_datetime(row.created_at)
        ));
    }

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "subscribers.export",
        None,
        None,
        None,
    )
    .await;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"subscribers.csv\"",
            ),
        ],
        csv,
    )
        .into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct CsrfOnly {
    pub csrf_token: String,
}

fn escape_csv(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
