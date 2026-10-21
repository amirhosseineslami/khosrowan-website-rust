//! Admin dashboard.

use askama::Template;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::error::AppResult;
use crate::handlers::admin::{admin_context, back_to};
use crate::handlers::data;
use crate::state::AppState;
use crate::util;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
pub struct DashboardTemplate {
    pub view: DashboardView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(&state, &auth.0, &headers, "/admin", "dashboard").await?;

    let stats = load_stats(&state).await?;

    // Recent posts
    let rows: Vec<crate::models::db::Post> =
        sqlx::query_as("SELECT * FROM posts ORDER BY updated_at DESC LIMIT 6")
            .fetch_all(&state.db)
            .await?;
    let recent_posts: Vec<AdminPostRow> = rows
        .iter()
        .map(|p| AdminPostRow {
            id: p.id,
            title: p.title.clone(),
            slug: p.slug.clone(),
            status: p.status.clone(),
            status_label: status_label(&p.status).to_string(),
            status_class: status_class(&p.status).to_string(),
            category: String::new(),
            views: p.view_count,
            comments: 0,
            published_at: util::format_date(p.published_at),
            updated_at: util::format_date(p.updated_at),
            is_featured: p.is_featured,
            scheduled_for: util::format_date(p.scheduled_for),
        })
        .collect();

    // Pending comments
    let comment_rows: Vec<crate::models::db::Comment> = sqlx::query_as(
        "SELECT c.*, p.title AS post_title, p.slug AS post_slug
         FROM comments c LEFT JOIN posts p ON p.id = c.post_id
         WHERE c.status = 'pending' ORDER BY c.created_at DESC LIMIT 6",
    )
    .fetch_all(&state.db)
    .await?;
    let pending_comments: Vec<AdminCommentRow> = comment_rows
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

    // New messages
    let message_rows: Vec<crate::models::db::Message> = sqlx::query_as(
        "SELECT * FROM messages WHERE status = 'new' ORDER BY created_at DESC LIMIT 6",
    )
    .fetch_all(&state.db)
    .await?;
    let new_messages: Vec<AdminMessageRow> = message_rows
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

    // Traffic for the last 14 days.
    let traffic: Vec<ChartPoint> = sqlx::query_as::<_, (String, i64)>(
        "SELECT day, COUNT(*) FROM page_views
         WHERE day >= date('now', '-13 days') GROUP BY day ORDER BY day ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(label, value)| ChartPoint { label, value })
    .collect();

    // Most visited pages.
    let top_paths: Vec<ChartPoint> = sqlx::query_as::<_, (String, i64)>(
        "SELECT path, COUNT(*) FROM page_views GROUP BY path ORDER BY COUNT(*) DESC LIMIT 8",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(label, value)| ChartPoint { label, value })
    .collect();

    // Top posts by views.
    let top_posts: Vec<AdminPostRow> = sqlx::query_as::<_, crate::models::db::Post>(
        "SELECT * FROM posts WHERE status = 'published' ORDER BY view_count DESC LIMIT 5",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .iter()
    .map(|p| AdminPostRow {
        id: p.id,
        title: p.title.clone(),
        slug: p.slug.clone(),
        status: p.status.clone(),
        status_label: status_label(&p.status).to_string(),
        status_class: status_class(&p.status).to_string(),
        category: String::new(),
        views: p.view_count,
        comments: 0,
        published_at: util::format_date(p.published_at),
        updated_at: util::format_date(p.updated_at),
        is_featured: p.is_featured,
        scheduled_for: util::format_date(p.scheduled_for),
    })
    .collect();

    // Rough client breakdown.
    let browser_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT CASE
            WHEN user_agent LIKE '%Firefox%' THEN 'Firefox'
            WHEN user_agent LIKE '%Chrome%' AND user_agent NOT LIKE '%Edg%' THEN 'Chrome'
            WHEN user_agent LIKE '%Safari%' AND user_agent NOT LIKE '%Chrome%' THEN 'Safari'
            WHEN user_agent LIKE '%Edg%' THEN 'Edge'
            WHEN user_agent LIKE '%bot%' OR user_agent LIKE '%crawler%' THEN 'Bots'
            ELSE 'Other' END AS kind,
            COUNT(*) FROM page_views GROUP BY kind ORDER BY COUNT(*) DESC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let browser_stats: Vec<ChartPoint> = browser_rows
        .into_iter()
        .map(|(label, value)| ChartPoint { label, value })
        .collect();

    let view = DashboardView {
        ctx,
        base,
        stats,
        recent_posts,
        pending_comments,
        new_messages,
        traffic,
        top_posts,
        top_paths,
        browser_stats,
    };

    Ok(crate::templates::Html(DashboardTemplate { view }).into_response())
}

/// Load the aggregated statistics shown on the dashboard.
pub async fn load_stats(state: &AppState) -> AppResult<AdminStats> {
    let posts_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let posts_published: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE status = 'published'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    let posts_draft: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE status = 'draft'")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let posts_scheduled: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE status = 'scheduled'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    let projects_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let pages_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pages")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let comments_pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE status = 'pending'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    let comments_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comments")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let messages_new: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE status = 'new'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    let messages_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let subscribers: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM subscribers WHERE status = 'active'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    let views_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM page_views")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let views_week: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM page_views WHERE created_at >= datetime('now', '-7 days')",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    let media_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let users_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let storage_bytes: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(size), 0) FROM media")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let _ = data::count_published_posts(&state.db).await?;

    Ok(AdminStats {
        posts_total,
        posts_published,
        posts_draft,
        posts_scheduled,
        projects_total,
        pages_total,
        comments_pending,
        comments_total,
        messages_new,
        messages_total,
        subscribers,
        views_total,
        views_week,
        media_count,
        users_count,
        storage_bytes,
    })
}

/// Human readable label for a post status.
pub fn status_label(status: &str) -> &'static str {
    match status {
        "published" => "Published",
        "draft" => "Draft",
        "scheduled" => "Scheduled",
        "archived" => "Archived",
        "pending" => "Pending",
        "approved" => "Approved",
        "spam" => "Spam",
        "trash" => "Trashed",
        "active" => "Active",
        "new" => "New",
        "read" => "Read",
        "unsubscribed" => "Unsubscribed",
        "sent" => "Sent",
        "owner" => "Owner",
        "editor" => "Editor",
        _ => "Unknown",
    }
}

/// CSS class used to colour a status badge.
pub fn status_class(status: &str) -> &'static str {
    match status {
        "published" | "approved" | "active" | "sent" => "success",
        "draft" | "pending" | "new" | "read" => "warning",
        "scheduled" | "unsubscribed" | "archived" => "info",
        "spam" | "trash" | "owner" => "danger",
        _ => "muted",
    }
}

/// Shared helper: redirect back to a path with a flash message.
pub fn flash_redirect(path: &str, kind: &str, message: &str) -> Response {
    back_to(path, kind, message)
}

/// Format a byte count for display.
pub fn format_bytes(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Format a number with separators (used in the admin tables).
pub fn number(value: i64) -> String {
    util::format_number(value)
}
