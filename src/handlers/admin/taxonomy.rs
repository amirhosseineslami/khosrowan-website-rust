//! Admin: tags and categories.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::posts::refresh_tag_counts;
use crate::handlers::admin::{admin_context, flash_redirect};
use crate::state::AppState;
use crate::util;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/taxonomy.html")]
pub struct TaxonomyTemplate {
    pub view: AdminTaxonomyView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) =
        admin_context(&state, &auth.0, &headers, "/admin/taxonomy", "taxonomy").await?;

    let tag_rows: Vec<crate::models::db::Tag> =
        sqlx::query_as("SELECT * FROM tags ORDER BY name ASC")
            .fetch_all(&state.db)
            .await?;
    let tags = tag_rows
        .iter()
        .map(|t| AdminTagRow {
            id: t.id,
            name: t.name.clone(),
            slug: t.slug.clone(),
            description: t.description.clone().unwrap_or_default(),
            color: t.color.clone().unwrap_or_else(|| "#6366f1".into()),
            post_count: t.post_count,
        })
        .collect();

    let category_rows: Vec<crate::models::db::Category> =
        sqlx::query_as("SELECT * FROM categories ORDER BY name ASC")
            .fetch_all(&state.db)
            .await?;
    let categories = category_rows
        .iter()
        .map(|c| AdminCategoryRow {
            id: c.id,
            name: c.name.clone(),
            slug: c.slug.clone(),
            description: c.description.clone().unwrap_or_default(),
            post_count: c.post_count,
        })
        .collect();

    let view = AdminTaxonomyView {
        ctx,
        base,
        tags,
        categories,
    };
    Ok(crate::templates::Html(TaxonomyTemplate { view }).into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct TagForm {
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub csrf_token: String,
}

pub async fn store_tag(
    State(state): State<AppState>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<TagForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let name = form.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::validation("A tag name is required."));
    }
    let slug = util::slugify(if form.slug.trim().is_empty() {
        &name
    } else {
        form.slug.trim()
    });

    sqlx::query(
        "INSERT INTO tags (name, slug, description, color) VALUES (?, ?, ?, ?)
         ON CONFLICT(slug) DO UPDATE SET name = excluded.name, description = excluded.description, color = excluded.color",
    )
    .bind(&name)
    .bind(&slug)
    .bind(non_empty(&form.description))
    .bind(non_empty(&form.color))
    .execute(&state.db)
    .await?;

    refresh_tag_counts(&state).await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "tag.create",
        Some("tag"),
        None,
        Some(&name),
    )
    .await;

    Ok(flash_redirect("/admin/taxonomy", "success", "Tag saved."))
}

pub async fn update_tag(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<TagForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let name = form.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::validation("A tag name is required."));
    }
    let slug = util::slugify(if form.slug.trim().is_empty() {
        &name
    } else {
        form.slug.trim()
    });

    sqlx::query("UPDATE tags SET name = ?, slug = ?, description = ?, color = ? WHERE id = ?")
        .bind(&name)
        .bind(&slug)
        .bind(non_empty(&form.description))
        .bind(non_empty(&form.color))
        .bind(id)
        .execute(&state.db)
        .await?;

    refresh_tag_counts(&state).await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "tag.update",
        Some("tag"),
        Some(&id.to_string()),
        Some(&name),
    )
    .await;

    Ok(flash_redirect("/admin/taxonomy", "success", "Tag updated."))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CsrfOnly>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    sqlx::query("DELETE FROM tags WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "tag.delete",
        Some("tag"),
        Some(&id.to_string()),
        None,
    )
    .await;

    Ok(flash_redirect("/admin/taxonomy", "success", "Tag deleted."))
}

#[derive(Debug, serde::Deserialize)]
pub struct CategoryForm {
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub csrf_token: String,
}

pub async fn store_category(
    State(state): State<AppState>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CategoryForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let name = form.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::validation("A category name is required."));
    }
    let slug = util::slugify(if form.slug.trim().is_empty() {
        &name
    } else {
        form.slug.trim()
    });

    sqlx::query(
        "INSERT INTO categories (name, slug, description) VALUES (?, ?, ?)
         ON CONFLICT(slug) DO UPDATE SET name = excluded.name, description = excluded.description",
    )
    .bind(&name)
    .bind(&slug)
    .bind(non_empty(&form.description))
    .execute(&state.db)
    .await?;

    refresh_tag_counts(&state).await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "category.create",
        Some("category"),
        None,
        Some(&name),
    )
    .await;

    Ok(flash_redirect(
        "/admin/taxonomy",
        "success",
        "Category saved.",
    ))
}

pub async fn update_category(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CategoryForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let name = form.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::validation("A category name is required."));
    }
    let slug = util::slugify(if form.slug.trim().is_empty() {
        &name
    } else {
        form.slug.trim()
    });

    sqlx::query("UPDATE categories SET name = ?, slug = ?, description = ? WHERE id = ?")
        .bind(&name)
        .bind(&slug)
        .bind(non_empty(&form.description))
        .bind(id)
        .execute(&state.db)
        .await?;

    refresh_tag_counts(&state).await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "category.update",
        Some("category"),
        Some(&id.to_string()),
        Some(&name),
    )
    .await;

    Ok(flash_redirect(
        "/admin/taxonomy",
        "success",
        "Category updated.",
    ))
}

pub async fn delete_category(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CsrfOnly>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "category.delete",
        Some("category"),
        Some(&id.to_string()),
        None,
    )
    .await;

    Ok(flash_redirect(
        "/admin/taxonomy",
        "success",
        "Category deleted.",
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
