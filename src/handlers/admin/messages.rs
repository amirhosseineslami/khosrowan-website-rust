//! Admin: contact messages inbox.

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::error::AppResult;
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect, status_class};
use crate::state::AppState;
use crate::util;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/messages.html")]
pub struct MessagesTemplate {
    pub view: AdminMessagesView,
}

#[derive(Debug, Deserialize)]
pub struct MessagesQuery {
    pub status: Option<String>,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
    Query(query): Query<MessagesQuery>,
) -> AppResult<Response> {
    let (base, ctx) =
        admin_context(&state, &auth.0, &headers, "/admin/messages", "messages").await?;

    let status_filter = query.status.clone().unwrap_or_default();

    let rows: Vec<crate::models::db::Message> = if status_filter.is_empty()
        || status_filter == "all"
    {
        sqlx::query_as("SELECT * FROM messages ORDER BY created_at DESC LIMIT 200")
            .fetch_all(&state.db)
            .await?
    } else {
        sqlx::query_as("SELECT * FROM messages WHERE status = ? ORDER BY created_at DESC LIMIT 200")
            .bind(&status_filter)
            .fetch_all(&state.db)
            .await?
    };

    let messages = rows
        .iter()
        .map(|m| AdminMessageRow {
            id: m.id,
            name: m.name.clone(),
            email: m.email.clone(),
            subject: m.subject.clone().unwrap_or_default(),
            body: m.body.clone(),
            status: m.status.clone(),
            status_class: status_class(&m.status).to_string(),
            created_at: util::format_date(m.created_at),
            created_iso: util::format_datetime(m.created_at),
            ip: m.ip.clone().unwrap_or_default(),
        })
        .collect();

    let counts: Vec<ChartPoint> = sqlx::query_as::<_, (String, i64)>(
        "SELECT status, COUNT(*) FROM messages GROUP BY status ORDER BY COUNT(*) DESC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(label, value)| ChartPoint { label, value })
    .collect();

    let view = AdminMessagesView {
        ctx,
        base,
        messages,
        status_filter,
        counts,
    };

    Ok(crate::templates::Html(MessagesTemplate { view }).into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct StatusForm {
    pub status: String,
    pub csrf_token: String,
}

pub async fn change_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<StatusForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let status = match form.status.as_str() {
        "read" => "read",
        "archived" => "archived",
        "spam" => "spam",
        "new" => "new",
        _ => "new",
    };

    sqlx::query("UPDATE messages SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "message.status",
        Some("message"),
        Some(&id.to_string()),
        Some(status),
    )
    .await;

    Ok(flash_redirect(
        "/admin/messages",
        "success",
        "Message updated.",
    ))
}

pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<StatusForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    sqlx::query("DELETE FROM messages WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "message.delete",
        Some("message"),
        Some(&id.to_string()),
        None,
    )
    .await;

    Ok(flash_redirect(
        "/admin/messages",
        "success",
        "Message deleted.",
    ))
}
