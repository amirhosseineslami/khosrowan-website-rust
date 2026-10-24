//! Admin: media library.

use askama::Template;
use axum::extract::{Multipart, Path, State};
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect};
use crate::state::AppState;
use crate::util;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/media.html")]
pub struct MediaTemplate {
    pub view: AdminMediaView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(&state, &auth.0, &headers, "/admin/media", "media").await?;

    let rows: Vec<crate::models::db::MediaItem> =
        sqlx::query_as("SELECT * FROM media ORDER BY created_at DESC LIMIT 200")
            .fetch_all(&state.db)
            .await?;

    let items = rows
        .iter()
        .map(|m| AdminMediaRow {
            id: m.id,
            filename: m.filename.clone(),
            url: m.path.clone(),
            mime: m.mime.clone(),
            size: format_bytes(m.size),
            dimensions: match (m.width, m.height) {
                (Some(w), Some(h)) => format!("{w}×{h}"),
                _ => String::new(),
            },
            alt_text: m.alt_text.clone().unwrap_or_default(),
            created_at: util::format_date(m.created_at),
        })
        .collect();

    let total_size = format_bytes(rows.iter().map(|m| m.size).sum::<i64>());

    let view = AdminMediaView {
        ctx,
        base,
        items,
        total_size,
    };

    Ok(crate::templates::Html(MediaTemplate { view }).into_response())
}

/// Accept an uploaded file, store it on disk and record it in the library.
pub async fn upload(
    State(state): State<AppState>,
    auth: crate::extract::RequireAdmin,
    mut multipart: Multipart,
) -> AppResult<Response> {
    let mut csrf_token = String::new();
    let mut saved = 0usize;
    let mut errors: Vec<String> = Vec::new();

    // Buffer every submitted field first so we can verify the CSRF token
    // before any file is written to disk or recorded in the database.
    let mut pending: Vec<(String, String, Vec<u8>)> = Vec::new();

    let media_dir = state.config.media_dir.clone();
    std::fs::create_dir_all(&media_dir)?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("upload error: {e}")))?
    {
        let name = field.name().unwrap_or_default().to_string();

        if name == "csrf_token" {
            csrf_token = field.text().await.unwrap_or_default();
            continue;
        }

        if name != "file" {
            continue;
        }

        let filename = field
            .file_name()
            .map(|f| f.to_string())
            .unwrap_or_else(|| "upload".to_string());
        let content_type = field
            .content_type()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let bytes = field
            .bytes()
            .await
            .map_err(|e| AppError::bad_request(format!("could not read upload: {e}")))?;

        pending.push((filename, content_type, bytes.to_vec()));
    }

    if !csrf_token.is_empty() {
        verify_csrf(&auth.0, &csrf_token)?;
    } else {
        return Err(AppError::forbidden("Missing CSRF token."));
    }

    for (filename, content_type, bytes) in pending {
        if bytes.is_empty() {
            errors.push(format!("`{filename}` was empty."));
            continue;
        }
        if bytes.len() > state.config.max_upload_size {
            errors.push(format!(
                "`{filename}` is larger than the {} MB limit.",
                state.config.max_upload_size / (1024 * 1024)
            ));
            continue;
        }

        // Only allow a conservative set of media types. SVG and arbitrary text/
        // types are excluded because they are served inline from the same origin
        // and can carry scripts that would run in the admin context.
        let is_image = content_type.starts_with("image/") && content_type != "image/svg+xml";
        let is_pdf = content_type == "application/pdf";
        let is_zip = content_type == "application/zip";
        if !is_image && !is_pdf && !is_zip {
            errors.push(format!("`{filename}` has an unsupported content type."));
            continue;
        }

        // The stored extension and MIME must match the actual file content, not
        // the client-supplied filename or Content-Type. For images we decode the
        // bytes (which also validates them); for PDF/ZIP we pin the extension to
        // the declared type. This blocks the stored-XSS trick of naming a file
        // `x.html` while declaring `image/png`.
        let (extension, width, height) = if is_image {
            match image::load_from_memory(&bytes) {
                Ok(img) => {
                    let ext = match image::guess_format(&bytes) {
                        Ok(image::ImageFormat::Jpeg) => "jpg",
                        Ok(image::ImageFormat::Gif) => "gif",
                        Ok(image::ImageFormat::WebP) => "webp",
                        Ok(image::ImageFormat::Bmp) => "bmp",
                        Ok(image::ImageFormat::Tiff) => "tiff",
                        Ok(image::ImageFormat::Png) | Ok(_) => "png",
                        Err(_) => "png",
                    };
                    (ext.to_string(), Some(img.width() as i64), Some(img.height() as i64))
                }
                Err(_) => {
                    errors.push(format!("`{filename}` is not a valid image."));
                    continue;
                }
            }
        } else if is_pdf {
            ("pdf".to_string(), None, None)
        } else {
            ("zip".to_string(), None, None)
        };

        let base = util::sanitize_filename(
            std::path::Path::new(&filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("upload"),
        );
        let stored_name = format!(
            "{base}-{}.{extension}",
            &uuid::Uuid::new_v4().to_string()[..8]
        );
        let target = media_dir.join(&stored_name);

        std::fs::write(&target, &bytes)?;

        let public_path = format!("/media/{stored_name}");
        let mime = if is_pdf {
            "application/pdf"
        } else if is_zip {
            "application/zip"
        } else {
            "image/*"
        };

        sqlx::query(
            "INSERT INTO media (filename, stored_name, path, mime, size, width, height, uploaded_by, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&filename)
        .bind(&stored_name)
        .bind(&public_path)
        .bind(mime)
        .bind(bytes.len() as i64)
        .bind(width)
        .bind(height)
        .bind(auth.0.user_id)
        .bind(chrono::Utc::now())
        .execute(&state.db)
        .await?;

        saved += 1;
    }

    if saved > 0 {
        crate::audit(
            &state,
            Some(auth.0.user_id),
            "media.upload",
            Some("media"),
            None,
            Some(&format!("{saved} file(s)")),
        )
        .await;
        return Ok(flash_redirect(
            "/admin/media",
            "success",
            &format!("{saved} file(s) uploaded."),
        ));
    }

    let detail = errors.join(" ");
    Ok(flash_redirect(
        "/admin/media",
        "error",
        if detail.is_empty() {
            "Nothing was uploaded."
        } else {
            &detail
        },
    ))
}

pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CsrfOnly>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let row: Option<crate::models::db::MediaItem> =
        sqlx::query_as("SELECT * FROM media WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;

    let Some(row) = row else {
        return Err(AppError::not_found("That file does not exist."));
    };

    let file = state.config.media_dir.join(&row.stored_name);
    let _ = std::fs::remove_file(file);

    sqlx::query("DELETE FROM media WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "media.delete",
        Some("media"),
        Some(&id.to_string()),
        Some(&row.filename),
    )
    .await;

    Ok(flash_redirect("/admin/media", "success", "File deleted."))
}

#[derive(Debug, serde::Deserialize)]
pub struct AltForm {
    pub alt_text: String,
    pub csrf_token: String,
}

pub async fn update_alt(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<AltForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let alt = form.alt_text.trim();
    sqlx::query("UPDATE media SET alt_text = ? WHERE id = ?")
        .bind(if alt.is_empty() { None } else { Some(alt) })
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(flash_redirect(
        "/admin/media",
        "success",
        "Alt text updated.",
    ))
}

#[derive(Debug, serde::Deserialize)]
pub struct CsrfOnly {
    pub csrf_token: String,
}

pub fn format_bytes(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
