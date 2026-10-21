//! Builds the [`BaseContext`] that every page needs.

use axum::http::HeaderMap;
use chrono::Datelike;
use sqlx::SqlitePool;

use crate::auth::session::AuthContext;
use crate::error::AppResult;
use crate::models::db::SocialLink;
use crate::settings::Settings;
use crate::state::AppState;
use crate::util;
use crate::views::{BaseContext, FlashView, NavItem, SiteInfo, SocialView, UserView};

/// The primary navigation, before dynamic pages are appended.
fn primary_nav(current_path: &str) -> Vec<NavItem> {
    let items = [
        ("Home", "/"),
        ("About", "/about"),
        ("Projects", "/projects"),
        ("Blog", "/blog"),
        ("Experience", "/experience"),
        ("Services", "/services"),
        ("Contact", "/contact"),
    ];

    items
        .iter()
        .map(|(label, url)| NavItem {
            label: (*label).to_string(),
            url: (*url).to_string(),
            active: is_active(current_path, url),
        })
        .collect()
}

fn is_active(current_path: &str, url: &str) -> bool {
    if url == "/" {
        return current_path == "/";
    }
    current_path == url || current_path.starts_with(&format!("{url}/"))
}

/// Load the navigation pages that are flagged for the main menu.
async fn nav_pages(pool: &SqlitePool, current_path: &str) -> AppResult<Vec<NavItem>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT title, slug FROM pages WHERE status = 'published' AND show_in_nav = 1 ORDER BY nav_order ASC, title ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(title, slug)| {
            let url = format!("/{slug}");
            NavItem {
                label: title,
                active: is_active(current_path, &url),
                url,
            }
        })
        .collect())
}

/// Load the social links.
async fn socials(pool: &SqlitePool) -> AppResult<Vec<SocialView>> {
    let rows: Vec<SocialLink> =
        sqlx::query_as("SELECT * FROM social_links ORDER BY sort_order ASC")
            .fetch_all(pool)
            .await?;

    Ok(rows
        .into_iter()
        .map(|row| SocialView {
            platform: row.platform.clone(),
            url: row.url.clone(),
            label: row.label.clone().unwrap_or_else(|| row.platform.clone()),
            icon: icon_for(&row.platform),
        })
        .collect())
}

/// Map a platform name onto the inline SVG identifier used by the templates.
pub fn icon_for(platform: &str) -> String {
    match platform.to_ascii_lowercase().as_str() {
        "github" => "github".into(),
        "linkedin" => "linkedin".into(),
        "x" | "twitter" => "x".into(),
        "mastodon" => "mastodon".into(),
        "rss" => "rss".into(),
        "email" | "mail" => "mail".into(),
        "telegram" => "telegram".into(),
        "discord" => "discord".into(),
        "youtube" => "youtube".into(),
        "dribbble" => "dribbble".into(),
        "behance" => "behance".into(),
        "twitch" => "twitch".into(),
        _ => "link".into(),
    }
}

/// Build the shared page context.
pub async fn build(
    state: &AppState,
    auth: Option<&AuthContext>,
    headers: &HeaderMap,
    current_path: &str,
) -> AppResult<BaseContext> {
    let settings: Settings = state.settings.snapshot();

    let mut nav = primary_nav(current_path);
    nav.extend(nav_pages(&state.db, current_path).await?);

    let socials = socials(&state.db).await?;

    let flash = parse_flash(headers).unwrap_or_else(|| FlashView {
        kind: String::new(),
        message: String::new(),
    });

    let user = auth.map(|a| UserView {
        username: a.username.clone(),
        display_name: a.display_name.clone(),
        email: a.email.clone(),
        role: a.role.clone(),
        avatar: a.avatar.clone(),
        initials: util::initials(&a.display_name),
        is_owner: a.is_owner(),
    });

    Ok(BaseContext {
        site: SiteInfo {
            title: settings.title.clone(),
            subtitle: settings.subtitle.clone(),
            description: settings.description.clone(),
            tagline: settings.tagline.clone(),
            email: settings.email.clone(),
            phone: settings.phone.clone(),
            location: settings.location.clone(),
            available: settings.available,
            availability_note: settings.availability_note.clone(),
            logo_text: settings.logo_text.clone(),
            favicon: settings.favicon.clone(),
            cv_file: settings.cv_file.clone(),
        },
        nav,
        socials,
        user,
        flash,
        current_path: current_path.to_string(),
        canonical_url: format!(
            "{}{}",
            state.config.base_url.trim_end_matches('/'),
            current_path
        ),
        page_title: settings.title.clone(),
        meta_description: settings.seo_meta_description.clone(),
        og_image: settings.default_og_image.clone(),
        og_type: settings.seo_og_type.clone(),
        twitter_site: settings.seo_twitter_site.clone(),
        year: chrono::Utc::now().year(),
        accent: settings.theme_accent.clone(),
        accent2: settings.theme_accent_2.clone(),
        default_theme: settings.default_theme.clone(),
        enable_dark_mode: settings.enable_dark_mode,
        analytics_provider: settings.analytics_provider.clone(),
        plausible_domain: settings.analytics_plausible_domain.clone(),
        ga_id: settings.analytics_ga_measurement_id.clone(),
        analytics_head: settings.analytics_custom_head.clone(),
        analytics_footer: settings.analytics_custom_footer.clone(),
        google_verification: settings.seo_google_site_verification.clone(),
        bing_verification: settings.seo_bing_verification.clone(),
        base_url: state.config.base_url.clone(),
        rss_url: "/feed.xml".to_string(),
        sitemap_url: "/sitemap.xml".to_string(),
        csrf_token: auth.map(|a| a.csrf_token.clone()).unwrap_or_default(),
    })
}

/// Parse the `flash` query parameter used for redirect messages.
fn parse_flash(headers: &HeaderMap) -> Option<FlashView> {
    let query = headers.get(axum::http::header::REFERER)?.to_str().ok()?;
    let query = query.split_once('?').map(|(_, q)| q)?;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=')?;
        if key != "flash" {
            continue;
        }
        let value = percent_decode(value);
        let (kind, message) = value.split_once(':')?;
        return Some(FlashView {
            kind: kind.to_string(),
            message: message.to_string(),
        });
    }
    None
}

/// Minimal percent-decoding for query values.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
