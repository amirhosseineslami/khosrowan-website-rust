//! RSS feed, sitemap, robots.txt and JSON-LD structured data.

use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::error::AppResult;
use crate::markdown;
use crate::state::AppState;
use crate::util;

/// `GET /feed.xml` — an RSS 2.0 feed of the latest posts.
pub async fn rss(State(state): State<AppState>) -> AppResult<Response> {
    let settings = state.settings.snapshot();
    let base = state.config.base_url.trim_end_matches('/').to_string();

    let posts: Vec<crate::models::db::Post> = sqlx::query_as(
        "SELECT * FROM posts WHERE status = 'published' AND published_at <= ?
         ORDER BY published_at DESC LIMIT 30",
    )
    .bind(chrono::Utc::now())
    .fetch_all(&state.db)
    .await?;

    let mut items = String::new();
    for post in &posts {
        let url = format!("{base}/blog/{}", post.slug);
        let description = markdown::to_plain_text(&post.excerpt);
        items.push_str(&format!(
            "    <item>\n\
             \x20     <title>{title}</title>\n\
             \x20     <link>{url}</link>\n\
             \x20     <guid isPermaLink=\"true\">{url}</guid>\n\
             \x20     <description>{description}</description>\n\
             \x20     <pubDate>{pub_date}</pubDate>\n\
             \x20     <dc:creator>{creator}</dc:creator>\n\
             \x20   </item>\n",
            title = markdown::escape(&post.title),
            url = markdown::escape(&url),
            description = markdown::escape(&description),
            pub_date = post
                .published_at
                .map(|d| d.format("%a, %d %b %Y %H:%M:%S %z").to_string())
                .unwrap_or_default(),
            creator = markdown::escape(&settings.title),
        ));
    }

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:dc="http://purl.org/dc/elements/1.1/">
  <channel>
    <title>{title}</title>
    <link>{base}</link>
    <description>{description}</description>
    <language>en</language>
    <lastBuildDate>{now}</lastBuildDate>
    <atom:link href="{base}/feed.xml" rel="self" type="application/rss+xml"/>
{items}  </channel>
</rss>"#,
        title = markdown::escape(&settings.title),
        description = markdown::escape(&settings.seo_meta_description),
        now = chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S %z"),
    );

    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "application/rss+xml; charset=utf-8",
        )],
        xml,
    )
        .into_response())
}

/// `GET /sitemap.xml` — every publicly reachable URL.
pub async fn sitemap(State(state): State<AppState>) -> AppResult<Response> {
    let base = state.config.base_url.trim_end_matches('/').to_string();
    let settings = state.settings.snapshot();

    let mut entries: Vec<(String, String, String)> = Vec::new();

    let static_paths = [
        ("", "1.0", "daily"),
        ("about", "0.8", "monthly"),
        ("projects", "0.9", "weekly"),
        ("blog", "0.9", "daily"),
        ("experience", "0.7", "monthly"),
        ("skills", "0.6", "monthly"),
        ("services", "0.8", "monthly"),
        ("testimonials", "0.6", "monthly"),
        ("awards", "0.5", "monthly"),
        ("speaking", "0.5", "monthly"),
        ("publications", "0.5", "monthly"),
        ("gallery", "0.5", "monthly"),
        ("contact", "0.7", "monthly"),
        ("resume", "0.6", "monthly"),
    ];

    for (path, priority, frequency) in static_paths {
        entries.push((
            format!("{base}/{path}"),
            priority.to_string(),
            frequency.to_string(),
        ));
    }

    let posts: Vec<(String, String)> = sqlx::query_as(
        "SELECT slug, published_at FROM posts WHERE status = 'published' AND published_at <= ?",
    )
    .bind(chrono::Utc::now())
    .fetch_all(&state.db)
    .await?;
    for (slug, _published) in posts {
        entries.push((
            format!("{base}/blog/{slug}"),
            "0.8".into(),
            "monthly".into(),
        ));
    }

    let projects: Vec<String> =
        sqlx::query_scalar("SELECT slug FROM projects WHERE status = 'active'")
            .fetch_all(&state.db)
            .await?;
    for slug in projects {
        entries.push((
            format!("{base}/projects/{slug}"),
            "0.7".into(),
            "monthly".into(),
        ));
    }

    let pages: Vec<String> =
        sqlx::query_scalar("SELECT slug FROM pages WHERE status = 'published' AND show_in_nav = 1")
            .fetch_all(&state.db)
            .await?;
    for slug in pages {
        entries.push((format!("{base}/{slug}"), "0.6".into(), "monthly".into()));
    }

    let body = entries
        .iter()
        .map(|(url, priority, frequency)| {
            format!(
                "  <url>\n    <loc>{}</loc>\n    <changefreq>{}</changefreq>\n    <priority>{}</priority>\n  </url>\n",
                markdown::escape(url),
                frequency,
                priority
            )
        })
        .collect::<String>();

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
{body}</urlset>"#
    );

    let _ = settings;
    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "application/xml; charset=utf-8",
        )],
        xml,
    )
        .into_response())
}

/// `GET /robots.txt`
pub async fn robots(State(state): State<AppState>) -> Response {
    let base = state.config.base_url.trim_end_matches('/').to_string();
    let body = format!(
        "User-agent: *\n\
         Allow: /\n\
         Disallow: /admin\n\
         Disallow: /admin/\n\
         Disallow: /media/\n\
         \n\
         Sitemap: {base}/sitemap.xml\n"
    );
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        body,
    )
        .into_response()
}

/// `GET /manifest.webmanifest`
pub async fn manifest(State(state): State<AppState>) -> Response {
    let settings = state.settings.snapshot();
    let body = serde_json::json!({
        "name": settings.title,
        "short_name": settings.logo_text,
        "description": settings.description,
        "start_url": "/",
        "display": "standalone",
        "background_color": "#0b0d12",
        "theme_color": settings.theme_accent,
        "icons": [
            { "src": settings.favicon, "sizes": "any", "type": "image/svg+xml" }
        ]
    });
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "application/manifest+json",
        )],
        body.to_string(),
    )
        .into_response()
}

/// `GET /.well-known/security.txt`
pub async fn security_txt(State(state): State<AppState>) -> Response {
    let settings = state.settings.snapshot();
    let body = format!(
        "Contact: mailto:{email}\n\
         Expires: {expires}\n\
         Preferred-Languages: en\n\
         Canonical: {base}/.well-known/security.txt\n",
        email = settings.email,
        expires = (chrono::Utc::now() + chrono::Duration::days(365)).format("%Y-%m-%dT%H:%M:%SZ"),
        base = state.config.base_url.trim_end_matches('/'),
    );
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        body,
    )
        .into_response()
}

/// JSON-LD structured data describing the site owner.
pub fn json_ld(state: &AppState) -> String {
    let settings = state.settings.snapshot();
    let base = state.config.base_url.trim_end_matches('/');

    let data = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "Person",
        "name": settings.title,
        "jobTitle": settings.subtitle,
        "description": settings.description,
        "url": base,
        "email": format!("mailto:{}", settings.email),
        "url": base,
        "sameAs": [],
    });

    data.to_string()
}

/// A small helper for the `<time>` element.
pub fn iso_date(date: Option<chrono::DateTime<chrono::Utc>>) -> String {
    util::format_datetime(date)
}
