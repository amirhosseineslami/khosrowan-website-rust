//! Admin: site settings.

use askama::Template;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect};
use crate::settings;
use crate::state::AppState;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/settings.html")]
pub struct SettingsTemplate {
    pub view: AdminSettingsView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) =
        admin_context(&state, &auth.0, &headers, "/admin/settings", "settings").await?;
    let groups = load_groups(&state).await?;

    let view = AdminSettingsView {
        ctx,
        base,
        groups,
        csrf_token: auth.0.csrf_token.clone(),
        saved: false,
    };

    Ok(crate::templates::Html(SettingsTemplate { view }).into_response())
}

/// Load the settings grouped by their `group_name`.
async fn load_groups(state: &AppState) -> AppResult<Vec<AdminSettingGroup>> {
    let rows: Vec<crate::models::db::SettingRow> =
        sqlx::query_as("SELECT * FROM settings ORDER BY group_name, key")
            .fetch_all(&state.db)
            .await?;

    let mut groups: Vec<AdminSettingGroup> = Vec::new();
    for row in rows {
        let (label_en, label_fa) = setting_label(&row.key);
        let field = AdminSettingField {
            key: row.key.clone(),
            value: row.value.clone().unwrap_or_default(),
            value_type: row.value_type.clone(),
            group_name: row.group_name.clone(),
            group_label: group_label(&row.group_name).to_string(),
            label: row
                .label
                .clone()
                .unwrap_or_else(|| label_en.clone()),
            label_en,
            label_fa,
            help_text: row.help_text.clone().unwrap_or_default(),
        };

        match groups.iter_mut().find(|g| g.name == field.group_name) {
            Some(group) => group.fields.push(field),
            None => {
                let (label_en, label_fa) = group_labels(&field.group_name);
                groups.push(AdminSettingGroup {
                    name: field.group_name.clone(),
                    label: label_en.to_string(),
                    label_fa: label_fa.to_string(),
                    fields: vec![field],
                });
            }
        }
    }

    Ok(groups)
}

/// Accept `#rgb`, `#rrggbb` or `#rrggbbaa`.
fn is_valid_hex_color(s: &str) -> bool {
    let Some(hex) = s.strip_prefix('#') else {
        return false;
    };
    let len = hex.len();
    (len == 3 || len == 6 || len == 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Human readable name for a settings group, in English and Persian.
pub fn group_labels(group: &str) -> (&'static str, &'static str) {
    match group {
        "general" => ("General", "عمومی"),
        "appearance" => ("Appearance", "ظاهر"),
        "content" => ("Content", "محتوا"),
        "seo" => ("SEO", "سئو"),
        "analytics" => ("Analytics", "تحلیل آمار"),
        "contact" => ("Contact", "تماس"),
        "newsletter" => ("Newsletter", "خبرنامه"),
        _ => ("Other", "سایر"),
    }
}

/// English label for a settings group (kept for compatibility).
pub fn group_label(group: &str) -> &'static str {
    group_labels(group).0
}

/// Human-readable label for a setting key in English and Persian. Falls back to
/// the raw key so an unknown key never renders blank.
fn setting_label(key: &str) -> (String, String) {
    let (en, fa) = match key {
        "site.title" => ("Title", "عنوان سایت"),
        "site.title_en" => ("Title (English)", "عنوان (انگلیسی)"),
        "site.subtitle" => ("Subtitle", "زیرعنوان"),
        "site.subtitle_en" => ("Subtitle (English)", "زیرعنوان (انگلیسی)"),
        "site.tagline" => ("Tagline", "شعار"),
        "site.tagline_en" => ("Tagline (English)", "شعار (انگلیسی)"),
        "site.logo_text" => ("Logo text", "متن لوگو"),
        "site.logo_text_en" => ("Logo text (English)", "متن لوگو (انگلیسی)"),
        "site.description" => ("Description", "توضیحات"),
        "site.description_en" => ("Description (English)", "توضیحات (انگلیسی)"),
        "site.hero_headline" => ("Hero headline", "تیتر اصلی"),
        "site.hero_headline_en" => ("Hero headline (English)", "تیتر اصلی (انگلیسی)"),
        "site.hero_subheadline" => ("Hero subheadline", "زیرتیتر اصلی"),
        "site.hero_subheadline_en" => {
            ("Hero subheadline (English)", "زیرتیتر اصلی (انگلیسی)")
        }
        "site.email" => ("Email", "ایمیل"),
        "site.phone" => ("Phone", "تلفن"),
        "site.location" => ("Location", "موقعیت"),
        "site.location_en" => ("Location (English)", "موقعیت (انگلیسی)"),
        "site.available" => ("Available", "در دسترس"),
        "site.availability_note" => ("Availability note", "یادداشت در دسترس بودن"),
        "site.availability_note_en" => {
            ("Availability note (English)", "یادداشت در دسترس بودن (انگلیسی)")
        }
        "site.favicon" => ("Favicon", "نماد"),
        "site.default_og_image" => ("Default OG image", "تصویر پیش‌فرض شبکه‌های اجتماعی"),
        "site.cv_file" => ("CV file", "فایل رزومه"),
        "site.allow_registration" => ("Allow registration", "اجازه ثبت‌نام"),
        "site.enable_comments" => ("Enable comments", "فعال‌سازی نظرات"),
        "site.enable_newsletter" => ("Enable newsletter", "فعال‌سازی خبرنامه"),
        "site.posts_per_page" => ("Posts per page", "تعداد پست در هر صفحه"),
        "site.theme_accent" => ("Accent color", "رنگ اصلی"),
        "site.theme_accent_2" => ("Secondary accent color", "رنگ اصلی دوم"),
        "site.default_theme" => ("Default theme", "پوسته پیش‌فرض"),
        "site.enable_dark_mode" => ("Enable dark mode", "فعال‌سازی حالت تیره"),
        "site.default_locale" => ("Default locale", "زبان پیش‌فرض"),
        "contact.form_enabled" => ("Form enabled", "فعال بودن فرم"),
        "contact.notify_email" => ("Notify email", "ایمیل اطلاع‌رسانی"),
        "contact.success_message" => ("Success message", "پیام موفقیت"),
        "contact.success_message_en" => ("Success message (English)", "پیام موفقیت (انگلیسی)"),
        "newsletter.heading" => ("Heading", "عنوان"),
        "newsletter.heading_en" => ("Heading (English)", "عنوان (انگلیسی)"),
        "newsletter.subheading" => ("Subheading", "زیرعنوان"),
        "newsletter.subheading_en" => ("Subheading (English)", "زیرعنوان (انگلیسی)"),
        "analytics.provider" => ("Provider", "ارائه‌دهنده"),
        "analytics.ga_measurement_id" => ("GA measurement ID", "شناسه گوگل آنالیتیکس"),
        "analytics.plausible_domain" => ("Plausible domain", "دامنه Plausible"),
        "analytics.custom_head" => ("Custom head", "کد سفارشی هد"),
        "analytics.custom_footer" => ("Custom footer", "کد سفارشی فوتر"),
        "seo.meta_description" => ("Meta description", "توضیحات متا"),
        "seo.og_type" => ("OG type", "نوع اوپن‌گراف"),
        "seo.person_name" => ("Person name", "نام شخص"),
        "seo.person_name_en" => ("Person name (English)", "نام شخص (انگلیسی)"),
        "seo.google_site_verification" => ("Google site verification", "کد تأیید گوگل"),
        "seo.bing_verification" => ("Bing verification", "کد تأیید بینگ"),
        "seo.twitter_site" => ("Twitter site", "حساب توییتر"),
        _ => (key, key),
    };
    (en.to_string(), fa.to_string())
}

/// Update every setting from the submitted form.
pub async fn update(
    State(state): State<AppState>,
    auth: crate::extract::RequireOwner,
    body: String,
) -> AppResult<Response> {
    // The form is `application/x-www-form-urlencoded` with many fields.
    let parsed: std::collections::HashMap<String, String> = serde_urlencoded::from_str(&body)
        .map_err(|e| crate::error::AppError::bad_request(e.to_string()))?;

    verify_csrf(
        &auth.0,
        parsed.get("csrf_token").map(|s| s.as_str()).unwrap_or(""),
    )?;

    let rows: Vec<(String, Option<String>, String)> =
        sqlx::query_as("SELECT key, value, value_type FROM settings")
            .fetch_all(&state.db)
            .await?;

    let mut updated = 0usize;
    for (key, _existing, value_type) in rows {
        if key == "csrf_token" {
            continue;
        }
        let Some(raw) = parsed.get(&key) else {
            continue;
        };

        // Checkbox-style booleans are submitted as "on" or absent.
        let value = match value_type.as_str() {
            "bool" => {
                if raw == "on" || raw == "true" || raw == "1" {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            "number" => raw
                .trim()
                .parse::<f64>()
                .map(|v| v.to_string())
                .unwrap_or_else(|_| raw.clone()),
            "color" => {
                let trimmed = raw.trim();
                // Color values are injected verbatim into a global <style> block
                // on every page, so they must be a strict hex color.
                if !is_valid_hex_color(trimmed) {
                    return Err(AppError::validation(format!(
                        "`{key}` must be a hex color like #6366f1."
                    )));
                }
                trimmed.to_string()
            }
            _ => raw.clone(),
        };

        settings::set(&state.db, &key, &value).await?;
        updated += 1;
    }

    state.settings.refresh(&state.db).await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "settings.update",
        Some("settings"),
        None,
        Some(&format!("{updated} value(s)")),
    )
    .await;

    Ok(flash_redirect(
        "/admin/settings",
        "success",
        "Settings saved.",
    ))
}
