//! Admin: editable profile content (experiences, services, skills) with
//! per-locale translations and reordering.

use askama::Template;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, AppResult};
use crate::extract::verify_csrf;
use crate::handlers::admin::{admin_context, flash_redirect};
use crate::state::AppState;
use crate::views::*;

#[derive(Template)]
#[template(path = "admin/profile_content.html")]
pub struct ProfileContentTemplate {
    pub view: AdminProfileContentView,
}

pub async fn index(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::RequireAdmin,
) -> AppResult<Response> {
    let (base, ctx) = admin_context(
        &state,
        &auth.0,
        &headers,
        "/admin/profile-content",
        "profile-content",
    )
    .await?;
    let csrf_token = base.csrf_token.clone();

    // Experiences
    let exp_rows: Vec<crate::models::db::Experience> =
        sqlx::query_as("SELECT * FROM experiences ORDER BY sort_order ASC, id ASC")
            .fetch_all(&state.db)
            .await?;
    let mut experiences = Vec::with_capacity(exp_rows.len());
    for e in &exp_rows {
        let t: Option<crate::models::db::ExperienceTranslation> = sqlx::query_as(
            "SELECT * FROM experience_translations WHERE experience_id = ? AND locale = 'en'",
        )
        .bind(e.id)
        .fetch_optional(&state.db)
        .await?;
        experiences.push(AdminExperienceRow {
            id: e.id,
            title: e.title.clone(),
            company: e.company.clone(),
            location: e.location.clone().unwrap_or_default(),
            start_date: e.start_date.clone().unwrap_or_default(),
            end_date: e.end_date.clone().unwrap_or_default(),
            is_current: e.is_current,
            description: e.description.clone(),
            kind: e.kind.clone(),
            sort_order: e.sort_order,
            title_en: t.as_ref().map(|x| x.title.clone()).unwrap_or_default(),
            company_en: t.as_ref().map(|x| x.company.clone()).unwrap_or_default(),
            description_en: t.as_ref().map(|x| x.description.clone()).unwrap_or_default(),
        });
    }

    // Services
    let svc_rows: Vec<crate::models::db::Service> =
        sqlx::query_as("SELECT * FROM services ORDER BY sort_order ASC, id ASC")
            .fetch_all(&state.db)
            .await?;
    let mut services = Vec::with_capacity(svc_rows.len());
    for s in &svc_rows {
        let t: Option<crate::models::db::ServiceTranslation> = sqlx::query_as(
            "SELECT * FROM service_translations WHERE service_id = ? AND locale = 'en'",
        )
        .bind(s.id)
        .fetch_optional(&state.db)
        .await?;
        services.push(AdminServiceRow {
            id: s.id,
            title: s.title.clone(),
            description: s.description.clone(),
            icon: s.icon.clone().unwrap_or_default(),
            price_from: s.price_from.clone().unwrap_or_default(),
            features: s.features.clone().unwrap_or_default(),
            sort_order: s.sort_order,
            title_en: t.as_ref().map(|x| x.title.clone()).unwrap_or_default(),
            description_en: t.as_ref().map(|x| x.description.clone()).unwrap_or_default(),
            features_en: t.as_ref().and_then(|x| x.features.clone()).unwrap_or_default(),
        });
    }

    // Skills
    let skill_rows: Vec<crate::models::db::Skill> =
        sqlx::query_as("SELECT * FROM skills ORDER BY sort_order ASC, name ASC")
            .fetch_all(&state.db)
            .await?;
    let mut skills = Vec::with_capacity(skill_rows.len());
    for k in &skill_rows {
        let t: Option<crate::models::db::SkillTranslation> = sqlx::query_as(
            "SELECT * FROM skill_translations WHERE skill_id = ? AND locale = 'en'",
        )
        .bind(k.id)
        .fetch_optional(&state.db)
        .await?;
        skills.push(AdminSkillRow {
            id: k.id,
            name: k.name.clone(),
            category: k.category.clone(),
            level: k.level,
            years: k.years.map(|y| format!("{y}")).unwrap_or_default(),
            sort_order: k.sort_order,
            name_en: t.as_ref().map(|x| x.name.clone()).unwrap_or_default(),
            category_en: t.as_ref().map(|x| x.category.clone()).unwrap_or_default(),
        });
    }

    let view = AdminProfileContentView {
        ctx,
        base,
        csrf_token,
        experiences,
        services,
        skills,
        flash: crate::views::FlashView {
            kind: String::new(),
            message: String::new(),
        },
    };
    Ok(crate::templates::Html(ProfileContentTemplate { view }).into_response())
}

#[derive(Debug, serde::Deserialize)]
pub struct ProfileForm {
    pub entity_type: String,
    pub id: i64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub start_date: String,
    #[serde(default)]
    pub end_date: String,
    #[serde(default)]
    pub is_current: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub level: i64,
    #[serde(default)]
    pub years: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub price_from: String,
    #[serde(default)]
    pub features: String,
    #[serde(default)]
    pub sort_order: i64,
    // English translation fields
    #[serde(default)]
    pub title_en: String,
    #[serde(default)]
    pub company_en: String,
    #[serde(default)]
    pub description_en: String,
    #[serde(default)]
    pub features_en: String,
    #[serde(default)]
    pub name_en: String,
    #[serde(default)]
    pub category_en: String,
    #[serde(default)]
    pub csrf_token: String,
}

pub async fn store(
    State(state): State<AppState>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<ProfileForm>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;

    match form.entity_type.as_str() {
        "experience" => update_experience(&state, &form, &auth.0).await?,
        "service" => update_service(&state, &form, &auth.0).await?,
        "skill" => update_skill(&state, &form, &auth.0).await?,
        _ => return Err(AppError::validation("Unknown entity type.")),
    }

    Ok(flash_redirect("/admin/profile-content", "success", "Saved."))
}

pub async fn destroy(
    State(state): State<AppState>,
    Path((entity, id)): Path<(String, i64)>,
    auth: crate::extract::RequireAdmin,
    axum::extract::Form(form): axum::extract::Form<CsrfOnly>,
) -> AppResult<Response> {
    verify_csrf(&auth.0, &form.csrf_token)?;
    let table = match entity.as_str() {
        "experience" => "experiences",
        "service" => "services",
        "skill" => "skills",
        _ => return Err(AppError::validation("Unknown entity type.")),
    };
    let sql = format!("DELETE FROM {table} WHERE id = ?");
    let result = sqlx::query(sql.as_str()).bind(id).execute(&state.db).await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("That entry does not exist."));
    }
    Ok(flash_redirect("/admin/profile-content", "success", "Deleted."))
}

async fn update_experience(
    state: &AppState,
    form: &ProfileForm,
    auth: &crate::auth::session::AuthContext,
) -> AppResult<()> {
    let title = form.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::validation("A title is required."));
    }
    let is_current = form.is_current.is_some();
    let end_date = if is_current {
        None
    } else if form.end_date.trim().is_empty() {
        None
    } else {
        Some(form.end_date.trim().to_string())
    };

    let result = sqlx::query(
        "UPDATE experiences SET title = ?, company = ?, location = ?, start_date = ?, end_date = ?,
                is_current = ?, description = ?, kind = ?, sort_order = ?
         WHERE id = ?",
    )
    .bind(&title)
    .bind(form.company.trim())
    .bind(non_empty(&form.location))
    .bind(non_empty(&form.start_date))
    .bind(end_date)
    .bind(is_current)
    .bind(form.description.trim())
    .bind(if form.kind.is_empty() { "work" } else { form.kind.trim() })
    .bind(form.sort_order)
    .bind(form.id)
    .execute(&state.db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("That experience does not exist."));
    }

    // English translation
    if !form.title_en.is_empty() {
        sqlx::query(
            "INSERT OR REPLACE INTO experience_translations (experience_id, locale, title, company, location, description)
             VALUES (?, 'en', ?, ?, ?, ?)",
        )
        .bind(form.id)
        .bind(&form.title_en)
        .bind(if form.company_en.is_empty() { &title } else { &form.company_en })
        .bind(non_empty(&form.location))
        .bind(non_empty(&form.description_en))
        .execute(&state.db)
        .await?;
    }

    crate::audit(
        state,
        Some(auth.user_id),
        "experience.update",
        Some("experience"),
        Some(&form.id.to_string()),
        Some(&title),
    )
    .await;
    Ok(())
}

async fn update_service(
    state: &AppState,
    form: &ProfileForm,
    auth: &crate::auth::session::AuthContext,
) -> AppResult<()> {
    let title = form.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::validation("A title is required."));
    }

    let result = sqlx::query(
        "UPDATE services SET title = ?, description = ?, icon = ?, price_from = ?, features = ?, sort_order = ?
         WHERE id = ?",
    )
    .bind(&title)
    .bind(form.description.trim())
    .bind(non_empty(&form.icon))
    .bind(non_empty(&form.price_from))
    .bind(non_empty(&form.features))
    .bind(form.sort_order)
    .bind(form.id)
    .execute(&state.db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("That service does not exist."));
    }

    if !form.title_en.is_empty() {
        sqlx::query(
            "INSERT OR REPLACE INTO service_translations (service_id, locale, title, description, features)
             VALUES (?, 'en', ?, ?, ?)",
        )
        .bind(form.id)
        .bind(&form.title_en)
        .bind(non_empty(&form.description_en))
        .bind(non_empty(&form.features_en))
        .execute(&state.db)
        .await?;
    }

    crate::audit(
        state,
        Some(auth.user_id),
        "service.update",
        Some("service"),
        Some(&form.id.to_string()),
        Some(&title),
    )
    .await;
    Ok(())
}

async fn update_skill(
    state: &AppState,
    form: &ProfileForm,
    auth: &crate::auth::session::AuthContext,
) -> AppResult<()> {
    let name = form.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::validation("A skill name is required."));
    }
    let years = form.years.trim().parse::<f64>().ok();

    let result = sqlx::query(
        "UPDATE skills SET name = ?, category = ?, level = ?, years = ?, sort_order = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(if form.category.is_empty() { "other" } else { form.category.trim() })
    .bind(form.level.clamp(0, 100))
    .bind(years)
    .bind(form.sort_order)
    .bind(form.id)
    .execute(&state.db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("That skill does not exist."));
    }

    if !form.name_en.is_empty() {
        sqlx::query(
            "INSERT OR REPLACE INTO skill_translations (skill_id, locale, name, category)
             VALUES (?, 'en', ?, ?)",
        )
        .bind(form.id)
        .bind(&form.name_en)
        .bind(if form.category_en.is_empty() { &name } else { &form.category_en })
        .execute(&state.db)
        .await?;
    }

    crate::audit(
        state,
        Some(auth.user_id),
        "skill.update",
        Some("skill"),
        Some(&form.id.to_string()),
        Some(&name),
    )
    .await;
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub struct CsrfOnly {
    pub csrf_token: String,
}

fn non_empty(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}
