//! Admin: static pages.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect, status_class, status_label};
use crate::markdown;
use crate::state::AppState;
use crate::util;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/pages.html")]
pub struct PagesTemplate {
    pub view: AdminPagesView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(&state, &auth.0, &headers, "/admin/pages", "pages").await?;

    let rows: Vec<crate::models::db::Page> =
        sqlx::query_as("SELECT * FROM pages ORDER BY nav_order ASC, title ASC")
            .fetch_all(&state.db)
            .await?;

    let pages = rows
        .iter()
        .map(|p| AdminPageRow {
            id: p.id,
            title: p.title.clone(),
            slug: p.slug.clone(),
            status: p.status.clone(),
            status_class: status_class(&p.status).to_string(),
            show_in_nav: p.show_in_nav,
            nav_order: p.nav_order,
            updated_at: util::format_date(p.updated_at),
        })
        .collect();

    let view = AdminPagesView { ctx, base, pages };
    Ok(crate::templates::Html(PagesTemplate { view }).into_response())
}

#[derive(Template)]
#[template(path = "admin/page_form.html")]
pub struct PageFormTemplate {
    pub view: AdminPageFormView,
}

pub async fn create(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(&state, &auth.0, &headers, "/admin/pages/new", "pages").await?;
    let view = AdminPageFormView {
        ctx,
        base,
        page: None,
        is_new: true,
        csrf_token: auth.0.csrf_token.clone(),
        error: String::new(),
    };
    Ok(crate::templates::Html(PageFormTemplate { view }).into_response())
}

pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(
        &state,
        &auth.0,
        &headers,
        &format!("/admin/pages/{id}/edit"),
        "pages",
    )
    .await?;

    let row: Option<crate::models::db::Page> = sqlx::query_as("SELECT * FROM pages WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;

    let Some(row) = row else {
        return Err(AppError::not_found("That page does not exist."));
    };

    let page = AdminPageEdit {
        id: row.id,
        title: row.title.clone(),
        slug: row.slug.clone(),
        content_md: row.content_md.clone(),
        content_html: row.content_html.clone(),
        status: row.status.clone(),
        is_full_width: row.is_full_width,
        show_in_nav: row.show_in_nav,
        nav_order: row.nav_order,
        meta_title: row.meta_title.clone().unwrap_or_default(),
        meta_description: row.meta_description.clone().unwrap_or_default(),
        updated_at: util::format_datetime(row.updated_at),
    };

    let view = AdminPageFormView {
        ctx,
        base,
        page: Some(page),
        is_new: false,
        csrf_token: auth.0.csrf_token.clone(),
        error: String::new(),
    };
    Ok(crate::templates::Html(PageFormTemplate { view }).into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct PageForm {
    pub title: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub is_full_width: Option<String>,
    #[serde(default)]
    pub show_in_nav: Option<String>,
    #[serde(default)]
    pub nav_order: String,
    #[serde(default)]
    pub meta_title: String,
    #[serde(default)]
    pub meta_description: String,
    #[serde(default)]
    pub csrf_token: String,
}

pub async fn store(
    State(state): State<AppState>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<PageForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let title = form.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::validation("A title is required."));
    }

    let slug = util::slugify(if form.slug.trim().is_empty() {
        &title
    } else {
        form.slug.trim()
    });

    let html = markdown::to_html(&form.content);
    let status = if form.status == "published" {
        "published".to_string()
    } else {
        "draft".to_string()
    };

    sqlx::query(
        "INSERT INTO pages (slug, title, content_md, content_html, status, is_full_width, show_in_nav,
                            nav_order, meta_title, meta_description, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&slug)
    .bind(&title)
    .bind(&form.content)
    .bind(&html)
    .bind(&status)
    .bind(form.is_full_width.is_some())
    .bind(form.show_in_nav.is_some())
    .bind(form.nav_order.trim().parse::<i64>().unwrap_or(0))
    .bind(non_empty(&form.meta_title))
    .bind(non_empty(&form.meta_description))
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now())
    .execute(&state.db)
    .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "page.create",
        Some("page"),
        None,
        Some(&title),
    )
    .await;

    Ok(flash_redirect("/admin/pages", "success", "Page created."))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<PageForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let title = form.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::validation("A title is required."));
    }

    let slug = util::slugify(if form.slug.trim().is_empty() {
        &title
    } else {
        form.slug.trim()
    });
    let html = markdown::to_html(&form.content);
    let status = if form.status == "published" {
        "published".to_string()
    } else {
        "draft".to_string()
    };

    sqlx::query(
        "UPDATE pages SET slug = ?, title = ?, content_md = ?, content_html = ?, status = ?,
                is_full_width = ?, show_in_nav = ?, nav_order = ?, meta_title = ?, meta_description = ?,
                updated_at = ? WHERE id = ?",
    )
    .bind(&slug)
    .bind(&title)
    .bind(&form.content)
    .bind(&html)
    .bind(&status)
    .bind(form.is_full_width.is_some())
    .bind(form.show_in_nav.is_some())
    .bind(form.nav_order.trim().parse::<i64>().unwrap_or(0))
    .bind(non_empty(&form.meta_title))
    .bind(non_empty(&form.meta_description))
    .bind(chrono::Utc::now())
    .bind(id)
    .execute(&state.db)
    .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "page.update",
        Some("page"),
        Some(&id.to_string()),
        Some(&title),
    )
    .await;

    Ok(flash_redirect("/admin/pages", "success", "Page updated."))
}

pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CsrfOnly>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    sqlx::query("DELETE FROM pages WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "page.delete",
        Some("page"),
        Some(&id.to_string()),
        None,
    )
    .await;

    Ok(flash_redirect("/admin/pages", "success", "Page deleted."))
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

/// Keep the status label helpers in scope for the templates.
pub fn _unused(status: &str) -> &'static str {
    status_label(status)
}
