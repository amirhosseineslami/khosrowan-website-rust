//! Admin: newsletter campaigns.

use askama::Template;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect, status_class};
use crate::markdown;
use crate::state::AppState;
use crate::util;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/newsletter.html")]
pub struct NewsletterTemplate {
    pub view: AdminNewsletterView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) =
        admin_context(&state, &auth.0, &headers, "/admin/newsletter", "newsletter").await?;

    let rows: Vec<crate::models::db::Campaign> =
        sqlx::query_as("SELECT * FROM campaigns ORDER BY created_at DESC LIMIT 50")
            .fetch_all(&state.db)
            .await?;

    let campaigns = rows
        .iter()
        .map(|c| AdminCampaignRow {
            id: c.id,
            subject: c.subject.clone(),
            status: c.status.clone(),
            status_class: status_class(&c.status).to_string(),
            recipient_count: c.recipient_count,
            sent_at: util::format_date(c.sent_at),
            created_at: util::format_date(c.created_at),
        })
        .collect();

    let subscriber_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM subscribers WHERE status = 'active'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let view = AdminNewsletterView {
        ctx,
        base,
        campaigns,
        subscriber_count,
        csrf_token: auth.0.csrf_token.clone(),
        sent: false,
        error: String::new(),
    };

    Ok(crate::templates::Html(NewsletterTemplate { view }).into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct CampaignForm {
    pub subject: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub csrf_token: String,
    /// When set the campaign is only saved as a draft.
    #[serde(default)]
    pub save_only: Option<String>,
}

pub async fn send(
    State(state): State<AppState>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CampaignForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    let subject = form.subject.trim().to_string();
    let body = form.body.trim().to_string();

    if subject.is_empty() {
        return Err(AppError::validation("Please add a subject line."));
    }
    if body.is_empty() {
        return Err(AppError::validation("Please write the newsletter content."));
    }

    let html = markdown::to_html(&body);
    let settings = state.settings.snapshot();

    if form.save_only.is_some() {
        sqlx::query("INSERT INTO campaigns (subject, body_html, status) VALUES (?, ?, 'draft')")
            .bind(&subject)
            .bind(&html)
            .execute(&state.db)
            .await?;

        crate::audit(
            &state,
            Some(auth.0.user_id),
            "newsletter.save",
            Some("campaign"),
            None,
            Some(&subject),
        )
        .await;

        return Ok(flash_redirect(
            "/admin/newsletter",
            "success",
            "Campaign saved as a draft.",
        ));
    }

    let subscribers: Vec<(String, Option<String>)> =
        sqlx::query_as("SELECT email, name FROM subscribers WHERE status = 'active'")
            .fetch_all(&state.db)
            .await?;

    if subscribers.is_empty() {
        return Err(AppError::validation("There are no active subscribers."));
    }

    let wrapper = crate::mail::html_wrapper(&subject, &html, &settings.theme_accent);
    let text = crate::mail::html_to_text(&html);

    let mut sent = 0usize;
    let mut failed = 0usize;

    for (email, name) in &subscribers {
        let personalised = match name {
            Some(n) if !n.trim().is_empty() => text.replace("{{name}}", n.trim()),
            _ => text.replace("{{name}}", "there"),
        };

        match state
            .mailer
            .send(email, &subject, &personalised, Some(&wrapper))
            .await
        {
            Ok(()) => sent += 1,
            Err(err) => {
                failed += 1;
                tracing::warn!(error = %err, email = %email, "failed to send newsletter");
            }
        }
    }

    sqlx::query(
        "INSERT INTO campaigns (subject, body_html, status, recipient_count, sent_at)
         VALUES (?, ?, 'sent', ?, ?)",
    )
    .bind(&subject)
    .bind(&html)
    .bind(sent as i64)
    .bind(chrono::Utc::now())
    .execute(&state.db)
    .await?;

    crate::audit(
        &state,
        Some(auth.0.user_id),
        "newsletter.send",
        Some("campaign"),
        None,
        Some(&format!("{sent} sent, {failed} failed")),
    )
    .await;

    let message = if failed > 0 {
        format!("Sent to {sent} subscriber(s); {failed} failed.")
    } else {
        format!("Sent to {sent} subscriber(s).")
    };

    Ok(flash_redirect("/admin/newsletter", "success", &message))
}
