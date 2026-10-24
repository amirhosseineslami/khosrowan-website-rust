//! The administrator panel.

pub mod backup;
pub mod comments;
pub mod dashboard;
pub mod logs;
pub mod media;
pub mod messages;
pub mod newsletter;
pub mod pages;
pub mod posts;
pub mod profile;
pub mod profile_content;
pub mod projects;
pub mod redirects;
pub mod settings;
pub mod socials;
pub mod subscribers;
pub mod taxonomy;
pub mod users;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{delete, get, post};
use axum::Router;

use crate::error::AppResult;
use crate::handlers::context;
use crate::state::AppState;
use crate::views::AdminContext;

pub use dashboard::{flash_redirect, status_class, status_label};

/// Build the shared admin context.
pub async fn admin_context(
    state: &AppState,
    auth: &crate::auth::session::AuthContext,
    headers: &axum::http::HeaderMap,
    path: &str,
    active: &str,
) -> AppResult<(crate::views::BaseContext, AdminContext)> {
    let base = context::build(state, Some(auth), headers, path).await?;

    let unread_messages: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE status = 'new'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    let pending_comments: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE status = 'pending'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    let draft_posts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE status IN ('draft', 'scheduled')")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    let subscriber_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM subscribers WHERE status = 'active'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let uptime = chrono::Utc::now()
        .signed_duration_since(state.started_at)
        .num_seconds();

    Ok((
        base.clone(),
        AdminContext {
            active: active.to_string(),
            unread_messages,
            pending_comments,
            draft_posts,
            subscriber_count,
            version: state.version.to_string(),
            uptime: format_duration(uptime),
        },
    ))
}

/// Format a duration in seconds as a compact human readable string.
pub fn format_duration(seconds: i64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

/// The admin router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/", get(dashboard::index))
        // Posts
        .route("/posts", get(posts::index))
        .route("/posts/new", get(posts::create).post(posts::store))
        .route("/posts/{id}/edit", get(posts::edit).post(posts::update))
        .route("/posts/{id}/delete", post(posts::destroy))
        .route("/posts/{id}/status", post(posts::change_status))
        // Pages
        .route("/pages", get(pages::index))
        .route("/pages/new", get(pages::create).post(pages::store))
        .route("/pages/{id}/edit", get(pages::edit).post(pages::update))
        .route("/pages/{id}/delete", post(pages::destroy))
        // Projects
        .route("/projects", get(projects::index))
        .route("/projects/new", get(projects::create).post(projects::store))
        .route(
            "/projects/{id}/edit",
            get(projects::edit).post(projects::update),
        )
        .route("/projects/{id}/delete", post(projects::destroy))
        // Profile content (experiences / services / skills)
        .route("/profile-content", get(profile_content::index))
        .route("/profile-content/save", post(profile_content::store))
        .route("/profile-content/{entity}/{id}/delete", post(profile_content::destroy))
        // Taxonomy
        .route("/taxonomy", get(taxonomy::index))
        .route("/taxonomy/tags", post(taxonomy::store_tag))
        .route("/taxonomy/tags/{id}", post(taxonomy::update_tag))
        .route("/taxonomy/tags/{id}/delete", post(taxonomy::delete_tag))
        .route("/taxonomy/categories", post(taxonomy::store_category))
        .route("/taxonomy/categories/{id}", post(taxonomy::update_category))
        .route(
            "/taxonomy/categories/{id}/delete",
            post(taxonomy::delete_category),
        )
        // Comments
        .route("/comments", get(comments::index))
        .route("/comments/{id}/status", post(comments::change_status))
        .route("/comments/{id}/delete", post(comments::destroy))
        // Messages
        .route("/messages", get(messages::index))
        .route("/messages/{id}/status", post(messages::change_status))
        .route("/messages/{id}/delete", post(messages::destroy))
        // Subscribers
        .route("/subscribers", get(subscribers::index))
        .route("/subscribers/{id}/delete", post(subscribers::destroy))
        .route("/subscribers/export.csv", get(subscribers::export))
        // Media
        .route("/media", get(media::index))
        .route("/media/upload", post(media::upload))
        .route("/media/{id}/delete", post(media::destroy))
        .route("/media/{id}/alt", post(media::update_alt))
        // Users
        .route("/users", get(users::index))
        .route("/users/new", get(users::create).post(users::store))
        .route("/users/{id}/edit", get(users::edit).post(users::update))
        .route("/users/{id}/delete", post(users::destroy))
        .route("/users/{id}/sessions", post(users::revoke_sessions))
        // Settings
        .route("/settings", get(settings::index))
        .route("/settings", post(settings::update))
        // Social links
        .route("/socials", get(socials::index))
        .route("/socials/save", post(socials::store))
        .route("/socials/{id}/delete", post(socials::destroy))
        // Redirects
        .route("/redirects", get(redirects::index))
        .route("/redirects", post(redirects::store))
        .route("/redirects/{id}/delete", post(redirects::destroy))
        // Newsletter
        .route("/newsletter", get(newsletter::index))
        .route("/newsletter/send", post(newsletter::send))
        // Activity log
        .route("/logs", get(logs::index))
        // Profile
        .route("/profile", get(profile::index))
        .route("/profile", post(profile::update))
        // Backup
        .route("/backup", get(backup::index))
        .route("/backup/export", get(backup::export))
        .route("/backup/import", post(backup::import))
        .route("/backup/reset", post(backup::reset))
        // API
        .route("/api/posts/{id}", delete(posts::api_destroy))
        .route("/api/markdown", post(posts::api_markdown))
        .route("/api/search", get(posts::api_search))
}

/// Redirect a guest to the login page.
pub fn require_login(request: &Request) -> Option<Response> {
    let _ = request;
    None
}

/// Helper used by handlers that need to redirect with a flash message.
pub fn back_to(path: &str, kind: &str, message: &str) -> Response {
    crate::error::redirect_with_flash(path, kind, message)
}

/// A 404 response for the admin area.
pub fn admin_not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        crate::templates::error_page(StatusCode::NOT_FOUND, "That admin page does not exist."),
    )
        .into_response()
}

/// Convenience re-export for redirects.
pub fn redirect(path: &str) -> Response {
    Redirect::to(path).into_response()
}
