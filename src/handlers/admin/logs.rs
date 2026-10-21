//! Admin: activity log.

use askama::Template;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::error::AppResult;
use crate::handlers::admin::admin_context;
use crate::state::AppState;
use crate::util;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/logs.html")]
pub struct LogsTemplate {
    pub view: AdminLogsView,
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub page: Option<usize>,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
    Query(query): Query<LogsQuery>,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(&state, &auth.0, &headers, "/admin/logs", "logs").await?;

    let per_page = 50usize;
    let page = query.page.unwrap_or(1).max(1);
    let offset = ((page - 1) * per_page) as i64;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let rows: Vec<crate::models::db::AuditEntry> =
        sqlx::query_as("SELECT * FROM audit_log ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(per_page as i64)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

    let entries = rows
        .iter()
        .map(|row| AdminLogRow {
            id: row.id,
            username: row.username.clone().unwrap_or_else(|| "system".into()),
            action: row.action.clone(),
            entity_type: row.entity_type.clone().unwrap_or_default(),
            entity_id: row.entity_id.clone().unwrap_or_default(),
            details: row.details.clone().unwrap_or_default(),
            ip: row.ip.clone().unwrap_or_default(),
            created_at: util::format_datetime(row.created_at),
            created_iso: util::format_datetime(row.created_at),
        })
        .collect();

    let pagination = PaginationView::from_page(
        &util::Page::new(Vec::<()>::new(), page, per_page, total),
        "/admin/logs",
        "",
    );

    let view = AdminLogsView {
        ctx,
        base,
        entries,
        pagination,
    };

    Ok(crate::templates::Html(LogsTemplate { view }).into_response())
}
