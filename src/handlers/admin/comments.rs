//! Admin: comment moderation.

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
#[template(path = "admin/comments.html")]
pub struct CommentsTemplate {
    pub view: AdminCommentsView,
}

#[derive(Debug, Deserialize)]
pub struct CommentsQuery {
    pub status: Option<String>,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
    Query(query): Query<CommentsQuery>,
) -> AppResult<Response> {
    let (base, ctx) =
        admin_context(&state, &auth.0, &headers, "/admin/comments", "comments").await?;

    let status_filter = query.status.clone().unwrap_or_default();

    let rows: Vec<crate::models::db::Comment> =
        if status_filter.is_empty() || status_filter == "all" {
            sqlx::query_as(
                "SELECT c.*, p.title AS post_title, p.slug AS post_slug
             FROM comments c LEFT JOIN posts p ON p.id = c.post_id
             ORDER BY c.created_at DESC LIMIT 200",
            )
            .fetch_all(&state.db)
            .await?
        } else {
            sqlx::query_as(
                "SELECT c.*, p.title AS post_title, p.slug AS post_slug
             FROM comments c LEFT JOIN posts p ON p.id = c.post_id
             WHERE c.status = ? ORDER BY c.created_at DESC LIMIT 200",
            )
            .bind(&status_filter)
            .fetch_all(&state.db)
            .await?
        };

    let comments = rows
        .iter()
        .map(|c| AdminCommentRow {
            id: c.id,
            post_title: c.post_title.clone().unwrap_or_default(),
            post_slug: c.post_slug.clone().unwrap_or_default(),
            author_name: c.author_name.clone(),
            author_email: c.author_email.clone().unwrap_or_default(),
            body: c.body.clone(),
            status: c.status.clone(),
            status_class: status_class(&c.status).to_string(),
            created_at: util::format_date(c.created_at),
            created_iso: util::format_datetime(c.created_at),
            ip: c.ip.clone().unwrap_or_default(),
        })
        .collect();

    let counts: Vec<ChartPoint> = sqlx::query_as::<_, (String, i64)>(
        "SELECT status, COUNT(*) FROM comments GROUP BY status ORDER BY COUNT(*) DESC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(label, value)| ChartPoint { label, value, pct: 0 })
    .collect();

    let view = AdminCommentsView {
        ctx,
        base,
        comments,
        status_filter,
        counts,
    };

    Ok(crate::templates::Html(CommentsTemplate { view }).into_response())
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
        "approved" => "approved",
        "spam" => "spam",
        "trash" => "trash",
        "pending" => "pending",
        _ => "pending",
    };

    sqlx::query("UPDATE comments SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "comment.status",
        Some("comment"),
        Some(&id.to_string()),
        Some(status),
    )
    .await;

    Ok(flash_redirect(
        "/admin/comments",
        "success",
        "Comment updated.",
    ))
}

pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<StatusForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    sqlx::query("DELETE FROM comments WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "comment.delete",
        Some("comment"),
        Some(&id.to_string()),
        None,
    )
    .await;

    Ok(flash_redirect(
        "/admin/comments",
        "success",
        "Comment deleted.",
    ))
}
