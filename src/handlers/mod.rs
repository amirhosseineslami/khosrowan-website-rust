//! HTTP handlers.

pub mod admin;
pub mod auth;
pub mod blog;
pub mod context;
pub mod data;
pub mod feeds;
pub mod public;

use axum::extract::{Path, State};
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use chrono::Datelike;
use rust_embed::RustEmbed;
use tower_http::services::ServeDir;

/// The static asset bundle, embedded into the binary at build time.
#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Assets;

use crate::auth::session::AuthContext;
use crate::middleware;
use crate::state::AppState;

/// Build the complete application router.
pub fn router(state: AppState) -> Router {
    // Routes that anybody can reach. Mounted at the root (default locale) and
    // again under `/en` and `/fa` for canonical per-language addresses.
    let public = public_routes();

    // Admin routes that do not require a session.
    let admin_public = Router::new()
        .route("/login", get(auth::login_form).post(auth::login_submit))
        .route("/logout", get(auth::logout).post(auth::logout))
        .route(
            "/forgot-password",
            get(auth::forgot_form).post(auth::forgot_submit),
        )
        .route(
            "/reset-password",
            get(auth::reset_form).post(auth::reset_submit),
        )
        .route("/reset-password/{token}", get(auth::reset_form));

    // Admin routes that require a session.
    let admin_protected = admin::router().layer(axum::middleware::from_fn_with_state(
        state.clone(),
        require_admin,
    ));

    let admin = Router::new().merge(admin_public).merge(admin_protected);

    // Uploaded media lives on disk; static assets are embedded in the binary.
    let media = ServeDir::new(&state.config.media_dir);

    Router::new()
        .merge(public)
        .nest("/en", public_routes())
        .nest("/fa", public_routes())
        .nest(
            "/admin",
            Router::new()
                .merge(admin)
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    middleware::gate_admin_host,
                )),
        )
        .nest_service("/assets", get(serve_asset))
        .nest_service("/media", media)
        .fallback(fallback)
        .layer(axum::middleware::from_fn(middleware::resolve_locale_prefix))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::load_session,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::track_view,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::security::headers_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::ratelimit::enforce,
        ))
        .layer(tower_http::compression::CompressionLayer::new().gzip(true))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}

/// The public (non-admin) route set. Mounted at `/`, `/en` and `/fa`.
fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(public::home))
        .route("/about", get(public::about))
        .route("/projects", get(public::projects))
        .route("/projects/{slug}", get(public::project_detail))
        .route("/blog", get(blog::index))
        .route("/lang/{locale}", get(public::set_language))
        .route("/blog/{slug}", get(blog::show))
        .route("/blog/{slug}/comment", post(blog::comment))
        .route("/tags/{slug}", get(tag_redirect))
        .route("/categories/{slug}", get(category_redirect))
        .route("/experience", get(public::experience))
        .route("/skills", get(public::skills))
        .route("/services", get(public::services))
        .route("/testimonials", get(public::testimonials))
        .route("/awards", get(public::awards))
        .route("/speaking", get(public::speaking))
        .route("/publications", get(public::publications))
        .route("/gallery", get(public::gallery))
        .route(
            "/contact",
            get(public::contact_form).post(public::contact_submit),
        )
        .route("/resume", get(public::resume))
        .route("/search", get(public::search))
        .route("/newsletter/subscribe", post(public::subscribe))
        .route("/feed.xml", get(feeds::rss))
        .route("/rss.xml", get(feeds::rss))
        .route("/sitemap.xml", get(feeds::sitemap))
        .route("/robots.txt", get(feeds::robots))
        .route("/manifest.webmanifest", get(feeds::manifest))
        .route("/.well-known/security.txt", get(feeds::security_txt))
}

/// Serve a file from the embedded asset bundle.
async fn serve_asset(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut response = file.data.into_owned().into_response();
            let headers = response.headers_mut();
            headers.insert(
                header::CONTENT_TYPE,
                mime.as_ref()
                    .parse()
                    .unwrap_or(header::HeaderValue::from_static("application/octet-stream")),
            );
            headers.insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("public, max-age=3600"),
            );
            response
        }
        None => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "asset not found",
        )
            .into_response(),
    }
}

/// Redirect unauthenticated visitors to the login page.
async fn require_admin(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    if request.extensions().get::<AuthContext>().is_none() {
        let path = request.uri().path();
        let next_url = format!("/admin/login?next={}", urlencode(path));
        return Redirect::to(&next_url).into_response();
    }
    next.run(request).await
}

/// `/tags/{slug}` → the blog filtered by tag.
async fn tag_redirect(Path(slug): Path<String>) -> Response {
    Redirect::to(&format!("/blog?tag={}", urlencode(&slug))).into_response()
}

/// `/categories/{slug}` → the blog filtered by category.
async fn category_redirect(Path(slug): Path<String>) -> Response {
    Redirect::to(&format!("/blog?category={}", urlencode(&slug))).into_response()
}

/// Handle any URL that did not match a route: CMS pages, redirects and the
/// 404 page.
async fn fallback(
    State(state): State<AppState>,
    auth: crate::extract::MaybeAuth,
    uri: Uri,
) -> Response {
    let raw_path = uri.path();

    // Normalise the canonical language root (`/en/` → `/en`, `/fa/` → `/fa`).
    if raw_path == "/en/" {
        return Redirect::to("/en").into_response();
    }
    if raw_path == "/fa/" {
        return Redirect::to("/fa").into_response();
    }
    if raw_path == "/admin/" {
        return Redirect::to("/admin").into_response();
    }

    let path = raw_path.trim_start_matches('/').to_string();

    if !path.is_empty() && !path.contains('/') {
        // A managed redirect?
        let target: Option<(String, i64)> = sqlx::query_as(
            "SELECT to_path, status_code FROM redirects WHERE from_path = ? AND is_active = 1",
        )
        .bind(&format!("/{path}"))
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

        if let Some((to, code)) = target {
            let _ = sqlx::query("UPDATE redirects SET hits = hits + 1 WHERE from_path = ?")
                .bind(&format!("/{path}"))
                .execute(&state.db)
                .await;
            let status = StatusCode::from_u16(code as u16).unwrap_or(StatusCode::FOUND);
            return (status, [(header::LOCATION, to.as_str())], "").into_response();
        }

        // A CMS page?
        let page: Option<(i64, String, String, String)> =
            sqlx::query_as("SELECT id, title, content_html, status FROM pages WHERE slug = ?")
                .bind(&path)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten();

        // Resolve the requested locale from the `?lang=` query parameter, falling
        // back to the site default.
        let default_locale = state.settings.snapshot().default_locale;
        let requested_locale = uri
            .query()
            .and_then(|q| q.split('&').find_map(|p| p.strip_prefix("lang=")))
            .unwrap_or(&default_locale);
        let locale = if requested_locale == "en" { "en" } else { "fa" };

        if let Some((page_id, title, content_html, status)) = page {
            if status == "published" {
                // Prefer a translated variant for the active locale.
                let (title, content_html) = if locale != "fa" {
                    let translated: Option<(String, String)> = sqlx::query_as(
                        "SELECT title, content_html FROM page_translations WHERE page_id = ? AND locale = ?",
                    )
                    .bind(page_id)
                    .bind(locale)
                    .fetch_optional(&state.db)
                    .await
                    .ok()
                    .flatten();
                    match translated {
                        Some((t_title, t_html)) => (t_title, t_html),
                        None => (title, content_html),
                    }
                } else {
                    (title, content_html)
                };

                let mut base = crate::handlers::context::build(
                    &state,
                    auth.0.as_ref(),
                    &axum::http::HeaderMap::new(),
                    &format!("/{path}"),
                )
                .await
                .unwrap_or_else(|_| crate::views::BaseContext {
                    site: crate::views::SiteInfo {
                        title: state.settings.snapshot().title,
                        subtitle: String::new(),
                        description: String::new(),
                        tagline: String::new(),
                        email: String::new(),
                        phone: String::new(),
                        location: String::new(),
                        available: false,
                        availability_note: String::new(),
                        logo_text: String::new(),
                        favicon: String::new(),
                        cv_file: String::new(),
                        person_name: String::new(),
                        person_name_en: String::new(),
                        title_en: String::new(),
                        logo_text_en: String::new(),
                    },
                    lang: "fa".into(),
                    dir: "rtl".into(),
                    nav: Vec::new(),
                    socials: Vec::new(),
                    user: None,
                    flash: crate::views::FlashView {
                        kind: String::new(),
                        message: String::new(),
                    },
                    current_path: format!("/{path}"),
                    canonical_url: format!("/{path}"),
                    page_title: title.clone(),
                    meta_description: String::new(),
                    og_image: String::new(),
                    og_type: "website".into(),
                    twitter_site: String::new(),
                    year: chrono::Utc::now().year(),
                    accent: String::new(),
                    accent2: String::new(),
                    default_theme: "dark".into(),
                    enable_dark_mode: true,
                    analytics_provider: "none".into(),
                    plausible_domain: String::new(),
                    ga_id: String::new(),
                    analytics_head: String::new(),
                    analytics_footer: String::new(),
                    google_verification: String::new(),
                    bing_verification: String::new(),
                    base_url: state.config.base_url.clone(),
                    rss_url: "/feed.xml".into(),
                    sitemap_url: "/sitemap.xml".into(),
                    csrf_token: String::new(),
                });
                base.page_title = format!("{title} · {}", base.site.title);
                base.meta_description = base.site.description.clone();

                #[derive(askama::Template)]
                #[template(path = "pages/page.html")]
                struct PageTemplate {
                    view: crate::views::PageView,
                }

                let view = crate::views::PageView {
                    title,
                    content_html,
                    is_full_width: false,
                    base,
                };
                return crate::templates::Html(PageTemplate { view }).into_response();
            }
        }
    }

    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        crate::templates::error_page(
            StatusCode::NOT_FOUND,
            "The page you were looking for has moved or never existed.",
        ),
    )
        .into_response()
}

fn urlencode(input: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(input, NON_ALPHANUMERIC).to_string()
}
