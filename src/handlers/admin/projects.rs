//! Admin: projects.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect, status_class};
use crate::markdown;
use crate::state::AppState;
use crate::util;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/projects.html")]
pub struct ProjectsTemplate {
    pub view: AdminProjectsView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) =
        admin_context(&state, &auth.0, &headers, "/admin/projects", "projects").await?;

    let rows: Vec<crate::models::db::Project> =
        sqlx::query_as("SELECT * FROM projects ORDER BY sort_order ASC, id ASC")
            .fetch_all(&state.db)
            .await?;

    let projects = rows
        .iter()
        .map(|p| AdminProjectRow {
            id: p.id,
            title: p.title.clone(),
            slug: p.slug.clone(),
            status: p.status.clone(),
            status_class: status_class(&p.status).to_string(),
            tech: p
                .tech_stack
                .as_deref()
                .and_then(|t| serde_json::from_str::<Vec<String>>(t).ok())
                .unwrap_or_default(),
            is_featured: p.is_featured,
            sort_order: p.sort_order,
            updated_at: util::format_date(p.updated_at),
        })
        .collect();

    let view = AdminProjectsView {
        ctx,
        base,
        projects,
    };
    Ok(crate::templates::Html(ProjectsTemplate { view }).into_response())
}

#[derive(Template)]
#[template(path = "admin/project_form.html")]
pub struct ProjectFormTemplate {
    pub view: AdminProjectFormView,
}

pub async fn create(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) =
        admin_context(&state, &auth.0, &headers, "/admin/projects/new", "projects").await?;
    let view = AdminProjectFormView {
        ctx,
        base,
        project: None,
        is_new: true,
        csrf_token: auth.0.csrf_token.clone(),
        error: String::new(),
    };
    Ok(crate::templates::Html(ProjectFormTemplate { view }).into_response())
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
        &format!("/admin/projects/{id}/edit"),
        "projects",
    )
    .await?;

    let row: Option<crate::models::db::Project> =
        sqlx::query_as("SELECT * FROM projects WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;

    let Some(row) = row else {
        return Err(AppError::not_found("That project does not exist."));
    };

    let project = AdminProjectEdit {
        id: row.id,
        title: row.title.clone(),
        slug: row.slug.clone(),
        summary: row.summary.clone(),
        description_md: row.description_md.clone(),
        description_html: row.description_html.clone(),
        cover_image: row.cover_image.clone().unwrap_or_default(),
        repo_url: row.repo_url.clone().unwrap_or_default(),
        live_url: row.live_url.clone().unwrap_or_default(),
        client: row.client.clone().unwrap_or_default(),
        status: row.status.clone(),
        is_featured: row.is_featured,
        sort_order: row.sort_order,
        tech_stack: row.tech_stack.clone().unwrap_or_default(),
        meta_title: row.meta_title.clone().unwrap_or_default(),
        meta_description: row.meta_description.clone().unwrap_or_default(),
        updated_at: util::format_datetime(row.updated_at),
    };

    let view = AdminProjectFormView {
        ctx,
        base,
        project: Some(project),
        is_new: false,
        csrf_token: auth.0.csrf_token.clone(),
        error: String::new(),
    };
    Ok(crate::templates::Html(ProjectFormTemplate { view }).into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct ProjectForm {
    pub title: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub cover_image: String,
    #[serde(default)]
    pub repo_url: String,
    #[serde(default)]
    pub live_url: String,
    #[serde(default)]
    pub client: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub is_featured: Option<String>,
    #[serde(default)]
    pub sort_order: String,
    #[serde(default)]
    pub tech_stack: String,
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
    axum::extract::Form(form): axum::extract::Form<ProjectForm>,
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
    let html = markdown::to_html(&form.description);
    let tech = serde_json::to_string(&util::split_list(&form.tech_stack))
        .unwrap_or_else(|_| "[]".to_string());

    sqlx::query(
        "INSERT INTO projects (slug, title, summary, description_md, description_html, cover_image,
                               repo_url, live_url, client, status, is_featured, sort_order, tech_stack,
                               meta_title, meta_description, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&slug)
    .bind(&title)
    .bind(form.summary.trim())
    .bind(&form.description)
    .bind(&html)
    .bind(non_empty(&form.cover_image))
    .bind(non_empty(&form.repo_url))
    .bind(non_empty(&form.live_url))
    .bind(non_empty(&form.client))
    .bind(&form.status)
    .bind(form.is_featured.is_some())
    .bind(form.sort_order.trim().parse::<i64>().unwrap_or(0))
    .bind(&tech)
    .bind(non_empty(&form.meta_title))
    .bind(non_empty(&form.meta_description))
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now())
    .execute(&state.db)
    .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "project.create",
        Some("project"),
        None,
        Some(&title),
    )
    .await;

    Ok(flash_redirect(
        "/admin/projects",
        "success",
        "Project created.",
    ))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<ProjectForm>,
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
    let html = markdown::to_html(&form.description);
    let tech = serde_json::to_string(&util::split_list(&form.tech_stack))
        .unwrap_or_else(|_| "[]".to_string());

    sqlx::query(
        "UPDATE projects SET slug = ?, title = ?, summary = ?, description_md = ?, description_html = ?,
                cover_image = ?, repo_url = ?, live_url = ?, client = ?, status = ?, is_featured = ?,
                sort_order = ?, tech_stack = ?, meta_title = ?, meta_description = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&slug)
    .bind(&title)
    .bind(form.summary.trim())
    .bind(&form.description)
    .bind(&html)
    .bind(non_empty(&form.cover_image))
    .bind(non_empty(&form.repo_url))
    .bind(non_empty(&form.live_url))
    .bind(non_empty(&form.client))
    .bind(&form.status)
    .bind(form.is_featured.is_some())
    .bind(form.sort_order.trim().parse::<i64>().unwrap_or(0))
    .bind(&tech)
    .bind(non_empty(&form.meta_title))
    .bind(non_empty(&form.meta_description))
    .bind(chrono::Utc::now())
    .bind(id)
    .execute(&state.db)
    .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "project.update",
        Some("project"),
        Some(&id.to_string()),
        Some(&title),
    )
    .await;

    Ok(flash_redirect(
        "/admin/projects",
        "success",
        "Project updated.",
    ))
}

pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CsrfOnly>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "project.delete",
        Some("project"),
        Some(&id.to_string()),
        None,
    )
    .await;

    Ok(flash_redirect(
        "/admin/projects",
        "success",
        "Project deleted.",
    ))
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
