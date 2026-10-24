//! Admin: blog posts.

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect, status_class, status_label};
use crate::markdown;
use crate::state::AppState;
use crate::util;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/posts.html")]
pub struct PostsTemplate {
    pub view: AdminPostsView,
}

#[derive(Debug, Deserialize)]
pub struct PostsQuery {
    pub page: Option<usize>,
    pub status: Option<String>,
    pub q: Option<String>,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
    Query(query): Query<PostsQuery>,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(&state, &auth.0, &headers, "/admin/posts", "posts").await?;

    let status_filter = query.status.clone().unwrap_or_default();
    let search = query.q.clone().unwrap_or_default().trim().to_lowercase();
    let per_page = 20usize;
    let page = query.page.unwrap_or(1).max(1);
    let offset = ((page - 1) * per_page) as i64;

    let mut clauses: Vec<String> = vec!["1 = 1".into()];
    if !status_filter.is_empty() && status_filter != "all" {
        clauses.push("p.status = ?".into());
    }
    if !search.is_empty() {
        clauses.push("(LOWER(p.title) LIKE ? OR LOWER(p.slug) LIKE ?)".into());
    }
    let where_clause = clauses.join(" AND ");

    let pattern = format!("%{search}%");

    let count_sql = format!("SELECT COUNT(*) FROM posts p WHERE {where_clause}");
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    if !status_filter.is_empty() && status_filter != "all" {
        count_query = count_query.bind(&status_filter);
    }
    if !search.is_empty() {
        count_query = count_query.bind(&pattern).bind(&pattern);
    }
    let total = count_query.fetch_one(&state.db).await?;

    let list_sql = format!(
        "SELECT p.* FROM posts p WHERE {where_clause} ORDER BY p.updated_at DESC LIMIT ? OFFSET ?"
    );
    let mut list_query = sqlx::query_as::<_, crate::models::db::Post>(&list_sql);
    if !status_filter.is_empty() && status_filter != "all" {
        list_query = list_query.bind(&status_filter);
    }
    if !search.is_empty() {
        list_query = list_query.bind(&pattern).bind(&pattern);
    }
    let rows = list_query
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

    // Attach comment counts and category names.
    let mut posts: Vec<AdminPostRow> = Vec::new();
    for row in &rows {
        let category: Option<String> = match row.category_id {
            Some(id) => sqlx::query_scalar("SELECT name FROM categories WHERE id = ?")
                .bind(id)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten(),
            None => None,
        };
        let comments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE post_id = ?")
            .bind(row.id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

        posts.push(AdminPostRow {
            id: row.id,
            title: row.title.clone(),
            slug: row.slug.clone(),
            status: row.status.clone(),
            status_label: status_label(&row.status).to_string(),
            status_class: status_class(&row.status).to_string(),
            category: category.unwrap_or_default(),
            views: row.view_count,
            comments,
            published_at: util::format_date(row.published_at),
            updated_at: util::format_date(row.updated_at),
            is_featured: row.is_featured,
            scheduled_for: util::format_date(row.scheduled_for),
        });
    }

    let pagination = PaginationView::from_page(
        &util::Page::new(Vec::<()>::new(), page, per_page, total),
        "/admin/posts",
        &query_string(&query),
    );

    let view = AdminPostsView {
        ctx,
        base,
        posts,
        pagination,
        status_filter,
        query: query.q.unwrap_or_default(),
        total,
    };

    Ok(crate::templates::Html(PostsTemplate { view }).into_response())
}

fn query_string(query: &PostsQuery) -> String {
    let mut parts = Vec::new();
    if let Some(status) = &query.status {
        if !status.is_empty() && status != "all" {
            parts.push(format!("status={status}"));
        }
    }
    if let Some(q) = &query.q {
        if !q.is_empty() {
            parts.push(format!("q={}", urlencode(q)));
        }
    }
    parts.join("&")
}

fn urlencode(input: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(input, NON_ALPHANUMERIC).to_string()
}

// ---------------------------------------------------------------------------
// Create / edit form
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "admin/post_form.html")]
pub struct PostFormTemplate {
    pub view: AdminPostFormView,
}

pub async fn create(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(&state, &auth.0, &headers, "/admin/posts/new", "posts").await?;

    let view = AdminPostFormView {
        ctx,
        base,
        post: None,
        categories: load_categories(&state).await?,
        tags: load_tags(&state).await?,
        all_tags: String::new(),
        is_new: true,
        csrf_token: auth.0.csrf_token.clone(),
        error: String::new(),
        preview_html: String::new(),
    };

    Ok(crate::templates::Html(PostFormTemplate { view }).into_response())
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
        &format!("/admin/posts/{id}/edit"),
        "posts",
    )
    .await?;

    let row: Option<crate::models::db::Post> = sqlx::query_as("SELECT * FROM posts WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;

    let Some(row) = row else {
        return Err(AppError::not_found("That post does not exist."));
    };

    // Load the English translation, if present.
    let translation: Option<crate::models::db::PostTranslation> = sqlx::query_as(
        "SELECT * FROM post_translations WHERE post_id = ? AND locale = 'en'",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    let post = AdminPostEdit {
        id: row.id,
        title: row.title.clone(),
        slug: row.slug.clone(),
        excerpt: row.excerpt.clone(),
        content_md: row.content_md.clone(),
        content_html: row.content_html.clone(),
        status: row.status.clone(),
        category_id: row.category_id,
        category_id_str: row.category_id.map(|id| id.to_string()).unwrap_or_default(),
        cover_image: row.cover_image.clone().unwrap_or_default(),
        is_featured: row.is_featured,
        allow_comments: row.allow_comments,
        meta_title: row.meta_title.clone().unwrap_or_default(),
        meta_description: row.meta_description.clone().unwrap_or_default(),
        published_at: row
            .published_at
            .map(|d| d.format("%Y-%m-%dT%H:%M").to_string())
            .unwrap_or_default(),
        scheduled_for: row
            .scheduled_for
            .map(|d| d.format("%Y-%m-%dT%H:%M").to_string())
            .unwrap_or_default(),
        reading_time: row.reading_time,
        word_count: markdown::word_count(&row.content_md),
        view_count: row.view_count,
        created_at: util::format_datetime(row.created_at),
        updated_at: util::format_datetime(row.updated_at),
        title_en: translation.as_ref().map(|t| t.title.clone()).unwrap_or_default(),
        excerpt_en: translation.as_ref().map(|t| t.excerpt.clone()).unwrap_or_default(),
        content_md_en: translation.as_ref().map(|t| t.content_md.clone()).unwrap_or_default(),
        meta_title_en: translation.as_ref().and_then(|t| t.meta_title.clone()).unwrap_or_default(),
        meta_description_en: translation
            .as_ref()
            .and_then(|t| t.meta_description.clone())
            .unwrap_or_default(),
    };

    let view = AdminPostFormView {
        ctx,
        base,
        post: Some(post),
        categories: load_categories(&state).await?,
        tags: load_tags(&state).await?,
        all_tags: String::new(),
        is_new: false,
        csrf_token: auth.0.csrf_token.clone(),
        error: String::new(),
        preview_html: String::new(),
    };

    Ok(crate::templates::Html(PostFormTemplate { view }).into_response())
}

#[derive(Debug, Deserialize)]
pub struct PostForm {
    pub title: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub excerpt: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub category_id: Option<i64>,
    #[serde(default)]
    pub cover_image: String,
    #[serde(default)]
    pub tags: String,
    #[serde(default)]
    pub is_featured: Option<String>,
    #[serde(default)]
    pub allow_comments: Option<String>,
    #[serde(default)]
    pub meta_title: String,
    #[serde(default)]
    pub meta_description: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub scheduled_for: String,
    #[serde(default)]
    pub title_en: String,
    #[serde(default)]
    pub excerpt_en: String,
    #[serde(default)]
    pub content_en: String,
    #[serde(default)]
    pub meta_title_en: String,
    #[serde(default)]
    pub meta_description_en: String,
    #[serde(default)]
    pub csrf_token: String,
}

pub async fn store(
    State(state): State<AppState>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<PostForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let title = form.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::validation("A title is required."));
    }

    let slug = unique_slug(
        &state,
        if form.slug.trim().is_empty() {
            util::slugify(&title)
        } else {
            util::slugify(form.slug.trim())
        },
        None,
    )
    .await?;

    if let Some(category_id) = form.category_id {
        let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM categories WHERE id = ?")
            .bind(category_id)
            .fetch_optional(&state.db)
            .await?;
        if exists.is_none() {
            return Err(AppError::validation("The selected category does not exist."));
        }
    }

    let html = markdown::to_html(&form.content);
    let reading_time = markdown::reading_time(&form.content);

    let status = normalise_status(&form.status);
    let published_at = parse_datetime(&form.published_at).or_else(|| {
        if status == "published" {
            Some(chrono::Utc::now())
        } else {
            None
        }
    });
    let scheduled_for = parse_datetime(&form.scheduled_for);

    let result: Result<i64, sqlx::Error> = sqlx::query_scalar(
        "INSERT INTO posts (slug, title, excerpt, content_md, content_html, status, category_id, author_id,
                            is_featured, allow_comments, meta_title, meta_description, cover_image,
                            published_at, scheduled_for, reading_time, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&slug)
    .bind(&title)
    .bind(form.excerpt.trim())
    .bind(&form.content)
    .bind(&html)
    .bind(&status)
    .bind(form.category_id)
    .bind(auth.0.user_id)
    .bind(form.is_featured.is_some())
    .bind(form.allow_comments.is_some())
    .bind(non_empty(&form.meta_title))
    .bind(non_empty(&form.meta_description))
    .bind(non_empty(&form.cover_image))
    .bind(published_at)
    .bind(scheduled_for)
    .bind(reading_time)
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now())
    .fetch_one(&state.db)
    .await;

    let post_id = result.map_err(AppError::from)?;

    sync_tags(&state, post_id, &form.tags).await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "post.create",
        Some("post"),
        Some(&post_id.to_string()),
        Some(&title),
    )
    .await;

    Ok(flash_redirect(
        "/admin/posts",
        "success",
        "Post created successfully.",
    ))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<PostForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let title = form.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::validation("A title is required."));
    }

    let slug = unique_slug(
        &state,
        if form.slug.trim().is_empty() {
            util::slugify(&title)
        } else {
            util::slugify(form.slug.trim())
        },
        Some(id),
    )
    .await?;

    let html = markdown::to_html(&form.content);
    let reading_time = markdown::reading_time(&form.content);
    let status = normalise_status(&form.status);

    // Read the raw value so NULL/empty string doesn't trip the DateTime decoder.
    let existing_published: Option<String> =
        sqlx::query_scalar("SELECT published_at FROM posts WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let existing_published = existing_published
        .filter(|s| !s.is_empty())
        .and_then(|s| parse_datetime(&s));

    let published_at = parse_datetime(&form.published_at)
        .or(existing_published)
        .or(if status == "published" {
            Some(chrono::Utc::now())
        } else {
            None
        });
    let scheduled_for = parse_datetime(&form.scheduled_for);

    sqlx::query(
        "UPDATE posts SET slug = ?, title = ?, excerpt = ?, content_md = ?, content_html = ?, status = ?,
                category_id = ?, is_featured = ?, allow_comments = ?, meta_title = ?, meta_description = ?,
                cover_image = ?, published_at = ?, scheduled_for = ?, reading_time = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&slug)
    .bind(&title)
    .bind(form.excerpt.trim())
    .bind(&form.content)
    .bind(&html)
    .bind(&status)
    .bind(form.category_id)
    .bind(form.is_featured.is_some())
    .bind(form.allow_comments.is_some())
    .bind(non_empty(&form.meta_title))
    .bind(non_empty(&form.meta_description))
    .bind(non_empty(&form.cover_image))
    .bind(published_at)
    .bind(scheduled_for)
    .bind(reading_time)
    .bind(chrono::Utc::now())
    .bind(id)
    .execute(&state.db)
    .await?;

    sync_tags(&state, id, &form.tags).await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "post.update",
        Some("post"),
        Some(&id.to_string()),
        Some(&title),
    )
    .await;

    Ok(flash_redirect("/admin/posts", "success", "Post updated."))
}

pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CsrfForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let title: Option<String> = sqlx::query_scalar("SELECT title FROM posts WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let Some(title) = title else {
        return Err(AppError::not_found("That post does not exist."));
    };

    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "post.delete",
        Some("post"),
        Some(&id.to_string()),
        Some(&title),
    )
    .await;

    Ok(flash_redirect("/admin/posts", "success", "Post deleted."))
}

#[derive(Debug, Deserialize)]
pub struct CsrfForm {
    pub csrf_token: String,
}

#[derive(Debug, Deserialize)]
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

    let status = normalise_status(&form.status);
    let published_at = if status == "published" {
        Some(chrono::Utc::now())
    } else {
        None
    };

    sqlx::query(
        "UPDATE posts SET status = ?, published_at = COALESCE(?, published_at), updated_at = ? WHERE id = ?",
    )
    .bind(&status)
    .bind(published_at)
    .bind(chrono::Utc::now())
    .bind(id)
    .execute(&state.db)
    .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "post.status",
        Some("post"),
        Some(&id.to_string()),
        Some(&status),
    )
    .await;

    Ok(flash_redirect(
        "/admin/posts",
        "success",
        &format!("Post marked as {}.", status_label(&status).to_lowercase()),
    ))
}

// ---------------------------------------------------------------------------
// API endpoints
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct MarkdownForm {
    pub content: String,
    #[serde(default)]
    pub csrf_token: String,
}

#[derive(serde::Serialize)]
pub struct MarkdownResponse {
    pub html: String,
    pub reading_time: i64,
    pub word_count: usize,
}

pub async fn api_markdown(
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<MarkdownForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let html = markdown::to_html(&form.content);
    Ok(Json(MarkdownResponse {
        html,
        reading_time: markdown::reading_time(&form.content),
        word_count: markdown::word_count(&form.content),
    })
    .into_response())
}

#[derive(Debug, Deserialize)]
pub struct ApiSearchQuery {
    pub q: Option<String>,
}

pub async fn api_search(
    State(state): State<AppState>,
    _auth: crate::extract::RequireAdmin,
    Query(query): Query<ApiSearchQuery>,
) -> AppResult<Response> {
    let term = query.q.unwrap_or_default().trim().to_lowercase();
    if term.is_empty() {
        return Ok(Json(Vec::<serde_json::Value>::new()).into_response());
    }
    let pattern = format!("%{term}%");

    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT title, slug, excerpt FROM posts WHERE LOWER(title) LIKE ? OR LOWER(excerpt) LIKE ?
         ORDER BY published_at DESC LIMIT 10",
    )
    .bind(&pattern)
    .bind(&pattern)
    .fetch_all(&state.db)
    .await?;

    let payload: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(title, slug, excerpt)| {
            serde_json::json!({ "title": title, "url": format!("/blog/{slug}"), "excerpt": excerpt })
        })
        .collect();

    Ok(Json(payload).into_response())
}

pub async fn api_destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    headers: axum::http::HeaderMap,
) -> AppResult<Response> {
    let token = headers
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    verify_csrf(&auth.0, token)?;

    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalise_status(status: &str) -> String {
    match status {
        "published" | "draft" | "scheduled" | "archived" => status.to_string(),
        _ => "draft".to_string(),
    }
}

fn parse_datetime(input: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(input) {
        return Some(dt.with_timezone(&chrono::Utc));
    }
    for format in [
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
    ] {
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(input, format) {
            return Some(naive.and_utc());
        }
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return Some(date.and_hms_opt(0, 0, 0)?.and_utc());
    }
    None
}

/// Build a slug that does not collide with an existing post.
async fn unique_slug(state: &AppState, base: String, exclude_id: Option<i64>) -> AppResult<String> {
    let mut candidate = base.clone();
    let mut counter = 2;

    loop {
        let existing: Option<i64> = sqlx::query_scalar("SELECT id FROM posts WHERE slug = ?")
            .bind(&candidate)
            .fetch_optional(&state.db)
            .await?;

        let collision = match (existing, exclude_id) {
            (Some(id), Some(exclude)) => id != exclude,
            (Some(_), None) => true,
            (None, _) => false,
        };

        if !collision {
            return Ok(candidate);
        }

        candidate = format!("{base}-{counter}");
        counter += 1;

        if counter > 500 {
            return Err(AppError::internal("could not generate a unique slug"));
        }
    }
}

/// Replace the tags of a post with the ones given in the form.
async fn sync_tags(state: &AppState, post_id: i64, tags: &str) -> AppResult<()> {
    let names: Vec<String> = tags
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    sqlx::query("DELETE FROM post_tags WHERE post_id = ?")
        .bind(post_id)
        .execute(&state.db)
        .await?;

    for name in names {
        let slug = util::slugify(&name);
        let tag_id: i64 = sqlx::query_scalar(
            "INSERT INTO tags (name, slug) VALUES (?, ?) ON CONFLICT(slug) DO UPDATE SET name = excluded.name RETURNING id",
        )
        .bind(&name)
        .bind(&slug)
        .fetch_one(&state.db)
        .await?;

        sqlx::query("INSERT OR IGNORE INTO post_tags (post_id, tag_id) VALUES (?, ?)")
            .bind(post_id)
            .bind(tag_id)
            .execute(&state.db)
            .await?;
    }

    refresh_tag_counts(state).await
}

/// Recalculate the cached tag and category post counts.
pub async fn refresh_tag_counts(state: &AppState) -> AppResult<()> {
    sqlx::query(
        "UPDATE tags SET post_count = (
            SELECT COUNT(*) FROM post_tags pt
            JOIN posts p ON p.id = pt.post_id
            WHERE pt.tag_id = tags.id AND p.status = 'published'
         )",
    )
    .execute(&state.db)
    .await?;

    sqlx::query(
        "UPDATE categories SET post_count = (
            SELECT COUNT(*) FROM posts p
            WHERE p.category_id = categories.id AND p.status = 'published'
         )",
    )
    .execute(&state.db)
    .await?;

    Ok(())
}

async fn load_categories(state: &AppState) -> AppResult<Vec<AdminCategoryRow>> {
    let rows: Vec<crate::models::db::Category> =
        sqlx::query_as("SELECT * FROM categories ORDER BY name ASC")
            .fetch_all(&state.db)
            .await?;
    Ok(rows
        .into_iter()
        .map(|c| AdminCategoryRow {
            id: c.id,
            name: c.name,
            slug: c.slug,
            description: c.description.unwrap_or_default(),
            post_count: c.post_count,
        })
        .collect())
}

async fn load_tags(state: &AppState) -> AppResult<Vec<AdminTagRow>> {
    let rows: Vec<crate::models::db::Tag> = sqlx::query_as("SELECT * FROM tags ORDER BY name ASC")
        .fetch_all(&state.db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|t| AdminTagRow {
            id: t.id,
            name: t.name,
            slug: t.slug,
            description: t.description.unwrap_or_default(),
            color: t.color.unwrap_or_else(|| "#6366f1".into()),
            post_count: t.post_count,
        })
        .collect())
}
