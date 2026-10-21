//! Blog listing, post detail and comments.

use askama::Template;
use axum::extract::{Form, Path, Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::handlers::context;
use crate::handlers::data;
use crate::markdown;
use crate::state::AppState;
use crate::util;
use crate::views::*;

// ---------------------------------------------------------------------------
// Listing
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "pages/blog.html")]
pub struct BlogTemplate {
    pub view: BlogIndexView,
}

#[derive(Debug, Deserialize)]
pub struct BlogQuery {
    pub page: Option<usize>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub q: Option<String>,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
    Query(query): Query<BlogQuery>,
) -> AppResult<Response> {
    let current_path = build_path(&query);
    let mut base = context::build(&state, auth.0.as_ref(), &headers, &current_path).await?;

    let settings = state.settings.snapshot();
    let per_page = settings.posts_per_page.clamp(1, 50);
    let page = query.page.unwrap_or(1).max(1);
    let offset = ((page - 1) * per_page) as i64;

    let category = query.category.clone().unwrap_or_default();
    let tag = query.tag.clone().unwrap_or_default();
    let term = query.q.clone().unwrap_or_default().trim().to_string();

    // Build the WHERE clause dynamically (all values are bound parameters).
    let mut clauses: Vec<String> = vec![
        "p.status = 'published'".into(),
        "p.published_at <= ?".into(),
    ];
    let mut category_id: Option<i64> = None;

    if !category.is_empty() {
        let found: Option<i64> = sqlx::query_scalar("SELECT id FROM categories WHERE slug = ?")
            .bind(&category)
            .fetch_optional(&state.db)
            .await?;
        if found.is_none() {
            return Err(AppError::not_found("That category does not exist."));
        }
        category_id = found;
        clauses.push("p.category_id = ?".into());
    }

    if !tag.is_empty() {
        clauses.push(
            "EXISTS (SELECT 1 FROM post_tags pt JOIN tags t ON t.id = pt.tag_id
                     WHERE pt.post_id = p.id AND t.slug = ?)"
                .into(),
        );
    }

    if !term.is_empty() {
        clauses.push(
            "(LOWER(p.title) LIKE ? OR LOWER(p.excerpt) LIKE ? OR LOWER(p.content_md) LIKE ?)"
                .into(),
        );
    }

    let where_clause = clauses.join(" AND ");

    // The bind order must match the order the placeholders appear.
    let pattern = format!("%{}%", term.to_lowercase());

    let count_sql = format!("SELECT COUNT(*) FROM posts p WHERE {where_clause}");
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql).bind(chrono::Utc::now());
    if category_id.is_some() {
        count_query = count_query.bind(category_id);
    }
    if !tag.is_empty() {
        count_query = count_query.bind(&tag);
    }
    if !term.is_empty() {
        count_query = count_query.bind(&pattern).bind(&pattern).bind(&pattern);
    }
    let total = count_query.fetch_one(&state.db).await?;

    let list_sql = format!(
        "SELECT p.id, p.slug, p.title, p.excerpt, p.cover_image, p.status, p.category_id,
                c.name AS category_name, c.slug AS category_slug, u.display_name AS author_name,
                p.is_featured, p.view_count, p.reading_time, p.published_at, p.updated_at
         FROM posts p
         LEFT JOIN categories c ON c.id = p.category_id
         LEFT JOIN users u ON u.id = p.author_id
         WHERE {where_clause}
         ORDER BY p.published_at DESC
         LIMIT ? OFFSET ?"
    );

    let mut list_query =
        sqlx::query_as::<_, crate::models::db::PostSummary>(&list_sql).bind(chrono::Utc::now());
    if category_id.is_some() {
        list_query = list_query.bind(category_id);
    }
    if !tag.is_empty() {
        list_query = list_query.bind(&tag);
    }
    if !term.is_empty() {
        list_query = list_query.bind(&pattern).bind(&pattern).bind(&pattern);
    }
    let summaries = list_query
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

    let mut posts: Vec<PostCard> = summaries.iter().map(PostCard::from_summary).collect();
    data::attach_tags(&state.db, &mut posts).await?;

    let pagination = PaginationView::from_page(
        &util::Page::new(Vec::<()>::new(), page, per_page, total),
        "/blog",
        &query_string(&query),
    );

    let heading = if !term.is_empty() {
        format!("Search results for “{term}”")
    } else if !category.is_empty() {
        category
            .split('-')
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    } else if !tag.is_empty() {
        format!("Posts tagged “{tag}”")
    } else {
        "Writing".to_string()
    };

    base.page_title = format!("{heading} · {}", base.site.title);

    let view = BlogIndexView {
        featured: Vec::new(),
        posts,
        pagination,
        categories: data::categories(&state.db).await?,
        tags: data::tags(&state.db).await?,
        heading,
        subheading:
            "Long-form notes on engineering, systems and the craft of building software that lasts."
                .into(),
        active_category: category,
        active_tag: tag,
        query: term,
        total,
        base,
    };

    Ok(crate::templates::Html(BlogTemplate { view }).into_response())
}

fn build_path(query: &BlogQuery) -> String {
    let mut path = String::from("/blog");
    let mut parts: Vec<String> = Vec::new();
    if let Some(c) = &query.category {
        parts.push(format!("category={c}"));
    }
    if let Some(t) = &query.tag {
        parts.push(format!("tag={t}"));
    }
    if let Some(q) = &query.q {
        parts.push(format!("q={}", urlencode(q)));
    }
    if !parts.is_empty() {
        path.push('?');
        path.push_str(&parts.join("&"));
    }
    path
}

fn query_string(query: &BlogQuery) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(c) = &query.category {
        parts.push(format!("category={}", urlencode(c)));
    }
    if let Some(t) = &query.tag {
        parts.push(format!("tag={}", urlencode(t)));
    }
    if let Some(q) = &query.q {
        parts.push(format!("q={}", urlencode(q)));
    }
    parts.join("&")
}

fn urlencode(input: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(input, NON_ALPHANUMERIC).to_string()
}

// ---------------------------------------------------------------------------
// Single post
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "pages/blog_post.html")]
pub struct BlogPostTemplate {
    pub view: BlogPostView,
}

pub async fn show(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let post = data::post_by_slug(&state.db, &slug).await?;

    let Some(post) = post else {
        return Err(AppError::not_found("That article does not exist."));
    };

    let now = chrono::Utc::now();
    let is_live =
        post.status == "published" && post.published_at.map(|p| p <= now).unwrap_or(false);
    if !is_live && auth.0.is_none() {
        return Err(AppError::not_found("That article does not exist."));
    }

    // Count the view (best effort, never blocks the response).
    let _ = sqlx::query("UPDATE posts SET view_count = view_count + 1 WHERE id = ?")
        .bind(post.id)
        .execute(&state.db)
        .await;

    let tags = tags_for_post(&state.db, post.id).await?;
    let category = match post.category_id {
        Some(id) => {
            sqlx::query_as::<_, (String, String)>("SELECT name, slug FROM categories WHERE id = ?")
                .bind(id)
                .fetch_optional(&state.db)
                .await?
                .map(|(name, slug)| (name, slug))
        }
        None => None,
    };

    let author: (String, Option<String>) =
        sqlx::query_as("SELECT display_name, bio FROM users WHERE id = ?")
            .bind(post.author_id)
            .fetch_optional(&state.db)
            .await?
            .unwrap_or_else(|| ("Anonymous".into(), None));

    let toc = markdown::table_of_contents(&post.content_md)
        .into_iter()
        .map(|entry| TocItem {
            level: entry.level,
            text: entry.text,
            anchor: entry.anchor,
        })
        .collect();

    let comments = load_comments(&state, post.id).await?;

    let (prev, next) = adjacent_posts(&state, post.id, post.published_at).await?;

    let related = related_posts(&state, post.id, post.category_id).await?;

    let mut base = context::build(
        &state,
        auth.0.as_ref(),
        &headers,
        &format!("/blog/{}", post.slug),
    )
    .await?;
    base.page_title = post
        .meta_title
        .clone()
        .unwrap_or_else(|| format!("{} · {}", post.title, base.site.title));
    base.meta_description = post
        .meta_description
        .clone()
        .unwrap_or_else(|| util::meta_description(&post.excerpt, &base.site.description));
    base.og_image = post
        .og_image
        .clone()
        .or_else(|| post.cover_image.clone())
        .unwrap_or(base.og_image);
    base.og_type = "article".into();

    let settings = state.settings.snapshot();

    let view = BlogPostView {
        post: PostView {
            slug: post.slug.clone(),
            title: post.title.clone(),
            excerpt: post.excerpt.clone(),
            content_html: post.content_html.clone(),
            cover_image: post.cover_image.clone().unwrap_or_default(),
            has_cover: post.cover_image.is_some(),
            category: category.as_ref().map(|c| c.0.clone()).unwrap_or_default(),
            category_slug: category.as_ref().map(|c| c.1.clone()).unwrap_or_default(),
            category_name: category.as_ref().map(|c| c.0.clone()).unwrap_or_default(),
            tags,
            author: author.0.clone(),
            author_initials: util::initials(&author.0),
            author_bio: author.1.unwrap_or_default(),
            published_at: util::format_date(post.published_at),
            published_iso: util::format_datetime(post.published_at),
            updated_at: util::format_date(post.updated_at),
            reading_time: post.reading_time,
            word_count: markdown::word_count(&post.content_md),
            view_count: post.view_count + 1,
            allow_comments: post.allow_comments && settings.enable_comments,
            toc,
            comments,
            prev,
            next,
            related,
            share_url: format!(
                "{}/blog/{}",
                state.config.base_url.trim_end_matches('/'),
                post.slug
            ),
        },
        comments_enabled: settings.enable_comments,
        comment_error: String::new(),
        comment_name: String::new(),
        comment_email: String::new(),
        comment_website: String::new(),
        base,
    };

    Ok(crate::templates::Html(BlogPostTemplate { view }).into_response())
}

async fn tags_for_post(pool: &sqlx::SqlitePool, post_id: i64) -> AppResult<Vec<TagView>> {
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT t.name, t.slug, t.color FROM post_tags pt JOIN tags t ON t.id = pt.tag_id
         WHERE pt.post_id = ? ORDER BY t.name ASC",
    )
    .bind(post_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(name, slug, color)| TagView {
            name,
            slug,
            color: color.unwrap_or_else(|| "#6366f1".into()),
            count: 0,
        })
        .collect())
}

async fn load_comments(state: &AppState, post_id: i64) -> AppResult<Vec<CommentView>> {
    let rows: Vec<crate::models::db::Comment> = sqlx::query_as(
        "SELECT c.*, p.title AS post_title, p.slug AS post_slug
         FROM comments c LEFT JOIN posts p ON p.id = c.post_id
         WHERE c.post_id = ? AND c.status = 'approved' ORDER BY c.created_at ASC",
    )
    .bind(post_id)
    .fetch_all(&state.db)
    .await?;

    let mut top_level: Vec<CommentView> = Vec::new();

    for row in rows {
        let view = CommentView {
            id: row.id,
            author_name: row.author_name.clone(),
            author_initials: util::initials(&row.author_name),
            author_website: row.author_website.clone().unwrap_or_default(),
            body: row.body.clone(),
            created_at: util::format_date(row.created_at),
            created_iso: util::format_datetime(row.created_at),
            time_ago: util::time_ago(row.created_at),
            replies: Vec::new(),
            depth: 0,
        };

        match row.parent_id {
            Some(parent_id) => {
                if let Some(parent) = find_comment_mut(&mut top_level, parent_id) {
                    parent.replies.push(CommentView { depth: 1, ..view });
                } else {
                    top_level.push(view);
                }
            }
            None => top_level.push(view),
        }
    }

    Ok(top_level)
}

fn find_comment_mut(list: &mut [CommentView], id: i64) -> Option<&mut CommentView> {
    for comment in list.iter_mut() {
        if comment.id == id {
            return Some(comment);
        }
        if let Some(found) = find_comment_mut(&mut comment.replies, id) {
            return Some(found);
        }
    }
    None
}

async fn adjacent_posts(
    state: &AppState,
    post_id: i64,
    published_at: Option<chrono::DateTime<chrono::Utc>>,
) -> AppResult<(Option<PostCard>, Option<PostCard>)> {
    let Some(published_at) = published_at else {
        return Ok((None, None));
    };

    let prev: Option<crate::models::db::PostSummary> = sqlx::query_as(
        "SELECT p.id, p.slug, p.title, p.excerpt, p.cover_image, p.status, p.category_id,
                c.name AS category_name, c.slug AS category_slug, u.display_name AS author_name,
                p.is_featured, p.view_count, p.reading_time, p.published_at, p.updated_at
         FROM posts p
         LEFT JOIN categories c ON c.id = p.category_id
         LEFT JOIN users u ON u.id = p.author_id
         WHERE p.status = 'published' AND p.published_at > ? AND p.id != ?
         ORDER BY p.published_at ASC LIMIT 1",
    )
    .bind(published_at)
    .bind(post_id)
    .fetch_optional(&state.db)
    .await?;

    let next: Option<crate::models::db::PostSummary> = sqlx::query_as(
        "SELECT p.id, p.slug, p.title, p.excerpt, p.cover_image, p.status, p.category_id,
                c.name AS category_name, c.slug AS category_slug, u.display_name AS author_name,
                p.is_featured, p.view_count, p.reading_time, p.published_at, p.updated_at
         FROM posts p
         LEFT JOIN categories c ON c.id = p.category_id
         LEFT JOIN users u ON u.id = p.author_id
         WHERE p.status = 'published' AND p.published_at < ? AND p.id != ?
         ORDER BY p.published_at DESC LIMIT 1",
    )
    .bind(published_at)
    .bind(post_id)
    .fetch_optional(&state.db)
    .await?;

    Ok((
        prev.as_ref().map(PostCard::from_summary),
        next.as_ref().map(PostCard::from_summary),
    ))
}

async fn related_posts(
    state: &AppState,
    post_id: i64,
    category_id: Option<i64>,
) -> AppResult<Vec<PostCard>> {
    let summaries: Vec<crate::models::db::PostSummary> = if let Some(category_id) = category_id {
        sqlx::query_as(
            "SELECT p.id, p.slug, p.title, p.excerpt, p.cover_image, p.status, p.category_id,
                    c.name AS category_name, c.slug AS category_slug, u.display_name AS author_name,
                    p.is_featured, p.view_count, p.reading_time, p.published_at, p.updated_at
             FROM posts p
             LEFT JOIN categories c ON c.id = p.category_id
             LEFT JOIN users u ON u.id = p.author_id
             WHERE p.status = 'published' AND p.category_id = ? AND p.id != ?
             ORDER BY p.published_at DESC LIMIT 3",
        )
        .bind(category_id)
        .bind(post_id)
        .fetch_all(&state.db)
        .await?
    } else {
        Vec::new()
    };

    let mut cards: Vec<PostCard> = summaries.iter().map(PostCard::from_summary).collect();
    data::attach_tags(&state.db, &mut cards).await?;
    Ok(cards)
}

// ---------------------------------------------------------------------------
// Comments
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CommentForm {
    pub author_name: String,
    pub author_email: String,
    pub author_website: Option<String>,
    pub body: String,
    #[serde(default)]
    pub parent_id: Option<i64>,
    /// Honeypot.
    #[serde(default)]
    pub website: Option<String>,
}

pub async fn comment(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: axum::http::HeaderMap,
    Form(form): Form<CommentForm>,
) -> AppResult<Response> {
    let ip = util::client_ip(&headers);
    let agent = util::user_agent(&headers);

    // Resolve the post from its slug.
    let post_id: Option<i64> = sqlx::query_scalar("SELECT id FROM posts WHERE slug = ?")
        .bind(&slug)
        .fetch_optional(&state.db)
        .await?;
    let Some(post_id) = post_id else {
        return Ok(Redirect::to("/blog").into_response());
    };

    // Honeypot.
    if form
        .website
        .as_deref()
        .map(|w| !w.is_empty())
        .unwrap_or(false)
    {
        return Ok(Redirect::to("/blog").into_response());
    }

    let name = form.author_name.trim().to_string();
    let email = form.author_email.trim().to_string();
    let body = form.body.trim().to_string();

    let mut errors: Vec<String> = Vec::new();
    if name.is_empty() || name.chars().count() > 120 {
        errors.push("Please provide a name.".into());
    }
    if !util::is_valid_email(&email) {
        errors.push("Please provide a valid e-mail address.".into());
    }
    if body.chars().count() < 3 {
        errors.push("Your comment is too short.".into());
    }
    if body.chars().count() > 5000 {
        errors.push("Your comment is too long.".into());
    }

    // Very short comments with links are usually spam.
    let link_count = body.matches("http").count();
    if body.chars().count() < 40 && link_count > 0 {
        errors.push("Your comment was flagged as spam.".into());
    }

    if !errors.is_empty() {
        let message = errors.join(" ");
        return Ok(
            Redirect::to(&format!("/blog?comment_error={}", urlencode(&message))).into_response(),
        );
    }

    let website = form
        .author_website
        .as_deref()
        .map(|w| w.trim())
        .filter(|w| !w.is_empty())
        .map(|w| {
            if w.starts_with("http://") || w.starts_with("https://") {
                w.to_string()
            } else {
                format!("https://{w}")
            }
        });

    sqlx::query(
        "INSERT INTO comments (post_id, parent_id, author_name, author_email, author_website, body, ip, user_agent, status)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending')",
    )
    .bind(post_id)
    .bind(form.parent_id)
    .bind(&name)
    .bind(&email)
    .bind(&website)
    .bind(&body)
    .bind(&ip)
    .bind(&agent)
    .execute(&state.db)
    .await?;

    Ok(Redirect::to("/blog?comment=pending").into_response())
}
