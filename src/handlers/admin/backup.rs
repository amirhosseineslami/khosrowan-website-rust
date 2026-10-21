//! Admin: backup, export and import.

use askama::Template;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::dashboard::load_stats;
use crate::handlers::admin::{admin_context, flash_redirect};
use crate::state::AppState;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/backup.html")]
pub struct BackupTemplate {
    pub view: AdminBackupView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireOwner,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(&state, &auth.0, &headers, "/admin/backup", "backup").await?;
    let stats = load_stats(&state).await?;

    let view = AdminBackupView {
        ctx,
        base,
        stats,
        csrf_token: auth.0.csrf_token.clone(),
    };

    Ok(crate::templates::Html(BackupTemplate { view }).into_response())
}

/// Export the whole content database as JSON.
pub async fn export(
    State(state): State<AppState>,
    auth: crate::extract::RequireOwner,
) -> AppResult<Response> {
    let payload = build_export(&state).await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "backup.export",
        None,
        None,
        None,
    )
    .await;

    let filename = format!(
        "personal-site-backup-{}.json",
        chrono::Utc::now().format("%Y-%m-%d")
    );

    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/json; charset=utf-8",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\"").as_str(),
            ),
        ],
        serde_json::to_string_pretty(&payload).unwrap_or_default(),
    )
        .into_response())
}

/// Collect every content table into a JSON document.
async fn build_export(state: &AppState) -> AppResult<serde_json::Value> {
    let tables = [
        "users",
        "posts",
        "categories",
        "tags",
        "post_tags",
        "projects",
        "pages",
        "comments",
        "messages",
        "subscribers",
        "media",
        "skills",
        "experiences",
        "services",
        "testimonials",
        "awards",
        "talks",
        "publications",
        "gallery",
        "social_links",
        "settings",
        "redirects",
    ];

    let mut export = serde_json::Map::new();
    export.insert(
        "exported_at".into(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );
    export.insert(
        "version".into(),
        serde_json::Value::String(state.version.to_string()),
    );

    use sqlx::Column;
    use sqlx::Row;

    for table in tables {
        let sql = format!("SELECT * FROM {table}");
        let Ok(result) = sqlx::query(&sql).fetch_all(&state.db).await else {
            export.insert(table.to_string(), serde_json::Value::Array(Vec::new()));
            continue;
        };

        let mut rows: Vec<serde_json::Value> = Vec::new();
        for row in &result {
            let mut object = serde_json::Map::new();
            for (index, column) in row.columns().iter().enumerate() {
                let value = if let Ok(v) = row.try_get::<Option<i64>, _>(index) {
                    v.map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null)
                } else if let Ok(v) = row.try_get::<Option<f64>, _>(index) {
                    v.map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null)
                } else if let Ok(v) = row.try_get::<Option<String>, _>(index) {
                    v.map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null)
                } else if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(index) {
                    v.map(|bytes| {
                        serde_json::Value::from(String::from_utf8_lossy(&bytes).to_string())
                    })
                    .unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                };
                object.insert(column.name().to_string(), value);
            }
            rows.push(serde_json::Value::Object(object));
        }

        export.insert(table.to_string(), serde_json::Value::Array(rows));
    }

    Ok(serde_json::Value::Object(export))
}

/// Import a previously exported document.
pub async fn import(
    State(state): State<AppState>,
    auth: crate::extract::RequireOwner,
    body: String,
) -> AppResult<Response> {
    let parsed: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| AppError::bad_request(format!("The file is not valid JSON: {e}")))?;

    let object = parsed
        .as_object()
        .ok_or_else(|| AppError::bad_request("The file must contain a JSON object."))?;

    let mut imported = 0usize;

    for (table, rows) in object {
        if !matches!(
            table.as_str(),
            "posts"
                | "categories"
                | "tags"
                | "post_tags"
                | "projects"
                | "pages"
                | "skills"
                | "experiences"
                | "services"
                | "testimonials"
                | "awards"
                | "talks"
                | "publications"
                | "gallery"
                | "social_links"
                | "settings"
                | "redirects"
        ) {
            continue;
        }

        let Some(rows) = rows.as_array() else {
            continue;
        };

        for row in rows {
            let Some(row) = row.as_object() else { continue };

            let columns: Vec<&str> = row.keys().map(|k| k.as_str()).collect();
            if columns.is_empty() {
                continue;
            }
            let placeholders = columns.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

            let sql = format!(
                "INSERT OR REPLACE INTO {table} ({}) VALUES ({})",
                columns.join(", "),
                placeholders
            );

            let mut query = sqlx::query(&sql);
            for column in &columns {
                let value = row.get(*column).cloned().unwrap_or(serde_json::Value::Null);
                query = match value {
                    serde_json::Value::String(s) => query.bind(s),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            query.bind(i)
                        } else {
                            query.bind(n.as_f64().unwrap_or(0.0))
                        }
                    }
                    serde_json::Value::Bool(b) => query.bind(b),
                    serde_json::Value::Null => query.bind(Option::<String>::None),
                    other => query.bind(other.to_string()),
                };
            }

            if query.execute(&state.db).await.is_ok() {
                imported += 1;
            }
        }
    }

    state.settings.refresh(&state.db).await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "backup.import",
        None,
        None,
        Some(&format!("{imported} row(s)")),
    )
    .await;

    Ok(flash_redirect(
        "/admin/backup",
        "success",
        &format!("Imported {imported} row(s)."),
    ))
}

/// Clear content tables (posts, pages, comments, messages) — a destructive
/// operation that requires an explicit confirmation string.
pub async fn reset(
    State(state): State<AppState>,
    auth: crate::extract::RequireOwner,
    body: String,
) -> AppResult<Response> {
    let parsed: std::collections::HashMap<String, String> =
        serde_urlencoded::from_str(&body).map_err(|e| AppError::bad_request(e.to_string()))?;

    verify_csrf(
        &auth.0,
        parsed.get("csrf_token").map(|s| s.as_str()).unwrap_or(""),
    )?;

    let confirmation = parsed.get("confirmation").map(|s| s.as_str()).unwrap_or("");
    if confirmation != "DELETE" {
        return Err(AppError::validation(
            "Type DELETE exactly to confirm this destructive operation.",
        ));
    }

    for table in [
        "post_tags",
        "comments",
        "posts",
        "pages",
        "messages",
        "page_views",
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&state.db)
            .await?;
    }

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "backup.reset",
        None,
        None,
        Some("content cleared"),
    )
    .await;

    Ok(flash_redirect(
        "/admin/backup",
        "success",
        "Content tables cleared.",
    ))
}
