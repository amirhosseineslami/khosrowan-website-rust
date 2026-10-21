//! Admin: site settings.

use askama::Template;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::error::AppResult;
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
        let field = AdminSettingField {
            key: row.key.clone(),
            value: row.value.clone().unwrap_or_default(),
            value_type: row.value_type.clone(),
            group_name: row.group_name.clone(),
            group_label: group_label(&row.group_name).to_string(),
            label: row.label.clone().unwrap_or_else(|| humanise(&row.key)),
            help_text: row.help_text.clone().unwrap_or_default(),
        };

        match groups.iter_mut().find(|g| g.name == field.group_name) {
            Some(group) => group.fields.push(field),
            None => groups.push(AdminSettingGroup {
                name: field.group_name.clone(),
                label: field.group_label.clone(),
                fields: vec![field],
            }),
        }
    }

    Ok(groups)
}

/// Human readable name for a settings group.
pub fn group_label(group: &str) -> &'static str {
    match group {
        "general" => "General",
        "appearance" => "Appearance",
        "content" => "Content",
        "seo" => "SEO",
        "analytics" => "Analytics",
        "contact" => "Contact",
        "newsletter" => "Newsletter",
        _ => "Other",
    }
}

/// Turn `site.default_theme` into `Site default theme`.
fn humanise(key: &str) -> String {
    let last = key.rsplit('.').next().unwrap_or(key);
    let mut chars = last.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => key.to_string(),
    }
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
