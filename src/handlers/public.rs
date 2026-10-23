//! Public-facing pages.

use askama::Template;
use axum::extract::{Form, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::handlers::context;
use crate::handlers::data;
use crate::state::AppState;
use crate::util;
use crate::views::*;

// ---------------------------------------------------------------------------
// Home
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "pages/home.html")]
pub struct HomeTemplate {
    pub view: HomeView,
}

pub async fn home(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/").await?;
    // For SEO, put the real (indexed) name first, then the brand.
    base.page_title = if base.lang == "fa" {
        format!("امیرحسین اسلامی · خسروان — پژوهش رباتیک و کنترل")
    } else {
        format!("Amirhossein Eslami · Khosrowan — Robotics & Control Research")
    };
    base.meta_description = base.site.description.clone();

    let stats = data::stats(&state.db, base.lang == "fa").await?;
    let mut featured_projects = data::project_cards(&state.db, "active").await?;
    data::apply_project_card_translations(&state.db, &mut featured_projects, &base.lang).await?;
    let featured_projects: Vec<ProjectCard> = featured_projects
        .into_iter()
        .filter(|p| p.is_featured)
        .take(3)
        .collect();
    let mut latest_posts = data::post_cards(&state.db, Some(3), Some(0)).await?;
    data::apply_post_card_translations(&state.db, &mut latest_posts, &base.lang).await?;
    let featured_posts: Vec<PostCard> = latest_posts
        .iter()
        .filter(|p| p.is_featured)
        .cloned()
        .collect();
    let latest_posts: Vec<PostCard> = latest_posts
        .into_iter()
        .filter(|p| !p.is_featured)
        .take(3)
        .collect();
    let skills = data::skill_groups(&state.db, &base.lang).await?;
    let services = data::services(&state.db, &base.lang).await?;
    let testimonials = data::testimonials(&state.db, true).await?;
    let timeline = data::experiences(&state.db, "work", &base.lang).await?;
    let blog_count = data::count_published_posts(&state.db).await?;
    let project_count = featured_projects.len() as i64;

    let hero_words: Vec<String> = if base.lang == "fa" {
        vec![
            "روباتیک".into(),
            "کنترل هوشمند".into(),
            "یادگیری ماشین".into(),
            "بینایی ماشین".into(),
            "مهندسی".into(),
        ]
    } else {
        vec![
            "robotics".into(),
            "intelligent control".into(),
            "machine learning".into(),
            "computer vision".into(),
            "engineering".into(),
        ]
    };

    let view = HomeView {
        hero_title: base.site.title.clone(),
        hero_highlight: base.site.subtitle.clone(),
        hero_subtitle: base.site.tagline.clone(),
        hero_words,
        availability_note: base.site.availability_note.clone(),
        stats,
        featured_projects,
        latest_posts,
        featured_posts,
        skills,
        services,
        testimonials,
        timeline,
        blog_count,
        project_count,
        base,
    };

    Ok(crate::templates::Html(HomeTemplate { view }).into_response())
}

// ---------------------------------------------------------------------------
// About
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "pages/about.html")]
pub struct AboutTemplate {
    pub view: AboutView,
}

pub async fn about(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/about").await?;
    base.page_title = if base.lang == "fa" {
        format!("درباره · {}", base.site.title)
    } else {
        format!("About · {}", base.site.title)
    };

    let content_html = load_page_html(&state, "about", &base.lang).await?;
    let stats = data::stats(&state.db, base.lang == "fa").await?;
    let skills = data::skill_groups(&state.db, &base.lang).await?;
    let timeline = data::experiences(&state.db, "work", &base.lang).await?;
    let services = data::services(&state.db, &base.lang).await?;
    let testimonials = data::testimonials(&state.db, true).await?;

    let facts = if base.lang == "fa" {
        vec![
            FactView {
                label: "سکونت".into(),
                value: base.site.location.clone(),
            },
            FactView {
                label: "تمرکز".into(),
                value: "معماری بک‌اند، راست، تجربه توسعه‌دهنده".into(),
            },
            FactView {
                label: "سبک کار".into(),
                value: "تیم‌های کوچک، اعتماد بالا، تصمیم‌های مکتوب".into(),
            },
            FactView {
                label: "باز پیشنهاد".into(),
                value: "مشاوره، راهنمایی و گاهی یک مسئله جذاب".into(),
            },
        ]
    } else {
        vec![
            FactView {
                label: "Currently".into(),
                value: base.site.location.clone(),
            },
            FactView {
                label: "Focus".into(),
                value: "Backend architecture, Rust, developer experience".into(),
            },
            FactView {
                label: "Working style".into(),
                value: "Small teams, high trust, written decisions".into(),
            },
            FactView {
                label: "Open to".into(),
                value: "Consulting, advisory and the occasional interesting problem".into(),
            },
        ]
    };

    let view = AboutView {
        content_html,
        stats,
        skills,
        timeline,
        facts,
        services,
        testimonials,
        photo_url: "/assets/img/profile.svg".into(),
        base,
    };

    Ok(crate::templates::Html(AboutTemplate { view }).into_response())
}

/// Load the rendered HTML of a CMS page, favouring the active locale's
/// translation when one exists, and falling back to the base row.
async fn load_page_html(state: &AppState, slug: &str, locale: &str) -> AppResult<String> {
    let page: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, content_html FROM pages WHERE slug = ? AND status = 'published'",
    )
    .bind(slug)
    .fetch_optional(&state.db)
    .await?;

    let Some((page_id, base_html)) = page else {
        return Ok(String::new());
    };

    if locale == "fa" {
        return Ok(base_html);
    }

    let translation: Option<String> = sqlx::query_scalar(
        "SELECT content_html FROM page_translations WHERE page_id = ? AND locale = ?",
    )
    .bind(page_id)
    .bind(locale)
    .fetch_optional(&state.db)
    .await?;

    Ok(translation.unwrap_or(base_html))
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "pages/projects.html")]
pub struct ProjectsTemplate {
    pub view: ProjectsView,
}

#[derive(Debug, Deserialize)]
pub struct ProjectsQuery {
    pub tech: Option<String>,
}

pub async fn projects(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
    Query(query): Query<ProjectsQuery>,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/projects").await?;
    base.page_title = if base.lang == "fa" {
        format!("پروژه‌ها · {}", base.site.title)
    } else {
        format!("Projects · {}", base.site.title)
    };

    let mut all = data::project_cards(&state.db, "active").await?;
    let total = all.len() as i64;

    let mut tech_list: Vec<String> = Vec::new();
    for project in &all {
        for item in &project.tech {
            if !tech_list.iter().any(|t| t.eq_ignore_ascii_case(item)) {
                tech_list.push(item.clone());
            }
        }
    }
    tech_list.sort();

    data::apply_project_card_translations(&state.db, &mut all, &base.lang).await?;

    let active_tech = query.tech.unwrap_or_default();
    let filtered: Vec<ProjectCard> = if active_tech.is_empty() {
        all
    } else {
        all.into_iter()
            .filter(|p| p.tech.iter().any(|t| t.eq_ignore_ascii_case(&active_tech)))
            .collect()
    };

    let view = ProjectsView {
        projects: filtered,
        tech: tech_list,
        active_tech,
        total,
        base,
    };

    Ok(crate::templates::Html(ProjectsTemplate { view }).into_response())
}

#[derive(Template)]
#[template(path = "pages/project_detail.html")]
pub struct ProjectDetailTemplate {
    pub view: ProjectDetailView,
}

pub async fn project_detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let project = data::project_by_slug(&state.db, &slug)
        .await?
        .filter(|p| p.status == "active")
        .ok_or_else(|| AppError::not_found("That project does not exist."))?;

    let mut base = context::build(
        &state,
        auth.0.as_ref(),
        &headers,
        &format!("/projects/{slug}"),
    )
    .await?;

    // Apply the active locale's translation, falling back to the base row.
    let translation = data::project_translation(&state.db, project.id, &base.lang).await?;
    let (title, summary, description_html) = if let Some(t) = &translation {
        (
            t.title.clone(),
            t.summary.clone(),
            t.description_html.clone(),
        )
    } else {
        (
            project.title.clone(),
            project.summary.clone(),
            project.description_html.clone(),
        )
    };

    base.page_title = format!("{} · {}", title, base.site.title);
    base.meta_description = project
        .meta_description
        .clone()
        .unwrap_or_else(|| summary.clone());
    base.og_image = project.cover_image.clone().unwrap_or(base.og_image);

    let tech = project
        .tech_stack
        .as_deref()
        .and_then(|t| serde_json::from_str::<Vec<String>>(t).ok())
        .unwrap_or_default();

    let mut all = data::project_cards(&state.db, "active").await?;
    data::apply_project_card_translations(&state.db, &mut all, &base.lang).await?;
    let related: Vec<ProjectCard> = all
        .into_iter()
        .filter(|p| p.slug != project.slug)
        .take(3)
        .collect();

    let view = ProjectDetailView {
        project: ProjectView {
            slug: project.slug.clone(),
            title,
            summary,
            description_html,
            cover_image: project.cover_image.clone().unwrap_or_default(),
            has_cover: project.cover_image.is_some(),
            repo_url: project.repo_url.clone().unwrap_or_default(),
            live_url: project.live_url.clone().unwrap_or_default(),
            client: project.client.clone().unwrap_or_default(),
            tech,
            tags: Vec::new(),
            created_at: util::format_date(project.created_at),
            updated_at: util::format_date(project.updated_at),
            related: Vec::new(),
            index: String::new(),
        },
        base,
    };

    let mut view = view;
    view.project.related = related;

    Ok(crate::templates::Html(ProjectDetailTemplate { view }).into_response())
}

// ---------------------------------------------------------------------------
// Experience / skills / services
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "pages/experience.html")]
pub struct ExperienceTemplate {
    pub view: ExperiencePageView,
}

pub async fn experience(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/experience").await?;
    base.page_title = if base.lang == "fa" {
        format!("تجربه و تحصیلات · {}", base.site.title)
    } else {
        format!("Experience · {}", base.site.title)
    };

    let view = ExperiencePageView {
        work: data::experiences(&state.db, "work", &base.lang).await?,
        education: data::experiences(&state.db, "education", &base.lang).await?,
        skills: data::skill_groups(&state.db, &base.lang).await?,
        stats: data::stats(&state.db, base.lang == "fa").await?,
        base,
    };

    Ok(crate::templates::Html(ExperienceTemplate { view }).into_response())
}

#[derive(Template)]
#[template(path = "pages/skills.html")]
pub struct SkillsTemplate {
    pub view: SkillsView,
}

pub async fn skills(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/skills").await?;
    base.page_title = if base.lang == "fa" {
        format!("مهارت‌ها · {}", base.site.title)
    } else {
        format!("Skills · {}", base.site.title)
    };

    let groups = data::skill_groups(&state.db, &base.lang).await?;
    let total = groups.iter().map(|g| g.items.len() as i64).sum();

    let view = SkillsView {
        skills: groups,
        total,
        base,
    };

    Ok(crate::templates::Html(SkillsTemplate { view }).into_response())
}

#[derive(Template)]
#[template(path = "pages/services.html")]
pub struct ServicesTemplate {
    pub view: ServicesView,
}

pub async fn services(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/services").await?;
    base.page_title = if base.lang == "fa" {
        format!("خدمات · {}", base.site.title)
    } else {
        format!("Services · {}", base.site.title)
    };

    let process = if base.lang == "fa" {
        vec![
            FactView {
                label: "۰۱ · کشف".into(),
                value: "در مورد مسئله، محدودیت‌ها و تعریف موفقیت گفت‌وگو می‌کنیم.".into(),
            },
            FactView {
                label: "۰۲ · طراحی".into(),
                value: "پیشنهاد مکتوب با معماری، برنامه و برآورد صادقانه.".into(),
            },
            FactView {
                label: "۰۳ · تحویل".into(),
                value: "اجرای عملی، در گام‌های کوچک قابل بازبینی.".into(),
            },
            FactView {
                label: "۰۴ · انتقال".into(),
                value: "مستندات، راهنمای گام‌به‌گام، و تیمی که دیگر به من نیاز ندارد.".into(),
            },
        ]
    } else {
        vec![
            FactView {
                label: "01 · Discover".into(),
                value: "We talk about the problem, the constraints and what success looks like.".into(),
            },
            FactView {
                label: "02 · Design".into(),
                value: "A written proposal with an architecture, a plan and honest estimates.".into(),
            },
            FactView {
                label: "03 · Deliver".into(),
                value: "Hands-on implementation, in small reviewable increments.".into(),
            },
            FactView {
                label: "04 · Hand over".into(),
                value: "Documentation, a walkthrough, and a team that no longer needs me.".into(),
            },
        ]
    };

    let view = ServicesView {
        services: data::services(&state.db, &base.lang).await?,
        process,
        testimonials: data::testimonials(&state.db, true).await?,
        base,
    };

    Ok(crate::templates::Html(ServicesTemplate { view }).into_response())
}

#[derive(Template)]
#[template(path = "pages/testimonials.html")]
pub struct TestimonialsTemplate {
    pub view: TestimonialsView,
}

pub async fn testimonials(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/testimonials").await?;
    base.page_title = if base.lang == "fa" {
        format!("نظرها · {}", base.site.title)
    } else {
        format!("Testimonials · {}", base.site.title)
    };

    let view = TestimonialsView {
        testimonials: data::testimonials(&state.db, false).await?,
        stats: data::stats(&state.db, base.lang == "fa").await?,
        base,
    };

    Ok(crate::templates::Html(TestimonialsTemplate { view }).into_response())
}

#[derive(Template)]
#[template(path = "pages/awards.html")]
pub struct AwardsTemplate {
    pub view: AwardsView,
}

pub async fn awards(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/awards").await?;
    base.page_title = if base.lang == "fa" {
        format!("جوایز و افتخارات · {}", base.site.title)
    } else {
        format!("Awards & Recognition · {}", base.site.title)
    };

    let view = AwardsView {
        awards: data::awards(&state.db).await?,
        talks: data::talks(&state.db).await?,
        publications: data::publications(&state.db).await?,
        base,
    };

    Ok(crate::templates::Html(AwardsTemplate { view }).into_response())
}

#[derive(Template)]
#[template(path = "pages/speaking.html")]
pub struct SpeakingTemplate {
    pub view: SpeakingView,
}

pub async fn speaking(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/speaking").await?;
    base.page_title = if base.lang == "fa" {
        format!("سخنرانی‌ها · {}", base.site.title)
    } else {
        format!("Speaking · {}", base.site.title)
    };

    let view = SpeakingView {
        talks: data::talks(&state.db).await?,
        publications: data::publications(&state.db).await?,
        base,
    };

    Ok(crate::templates::Html(SpeakingTemplate { view }).into_response())
}

#[derive(Template)]
#[template(path = "pages/publications.html")]
pub struct PublicationsTemplate {
    pub view: PublicationsView,
}

pub async fn publications(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/publications").await?;
    base.page_title = if base.lang == "fa" {
        format!("انتشارات · {}", base.site.title)
    } else {
        format!("Publications · {}", base.site.title)
    };

    let view = PublicationsView {
        publications: data::publications(&state.db).await?,
        talks: data::talks(&state.db).await?,
        base,
    };

    Ok(crate::templates::Html(PublicationsTemplate { view }).into_response())
}

#[derive(Template)]
#[template(path = "pages/gallery.html")]
pub struct GalleryTemplate {
    pub view: GalleryPageView,
}

pub async fn gallery(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
    Query(query): Query<GalleryQuery>,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/gallery").await?;
    base.page_title = if base.lang == "fa" {
        format!("گالری · {}", base.site.title)
    } else {
        format!("Gallery · {}", base.site.title)
    };

    let items = data::gallery(&state.db).await?;
    let mut categories: Vec<String> = Vec::new();
    for item in &items {
        if !item.category.is_empty() && !categories.contains(&item.category) {
            categories.push(item.category.clone());
        }
    }

    let active_category = query.category.unwrap_or_default();
    let filtered: Vec<GalleryViewItem> = if active_category.is_empty() {
        items
    } else {
        items
            .into_iter()
            .filter(|i| i.category.eq_ignore_ascii_case(&active_category))
            .collect()
    };

    let view = GalleryPageView {
        items: filtered,
        categories,
        active_category,
        base,
    };

    Ok(crate::templates::Html(GalleryTemplate { view }).into_response())
}

#[derive(Debug, Deserialize)]
pub struct GalleryQuery {
    pub category: Option<String>,
}

// ---------------------------------------------------------------------------
// Dynamic CMS pages
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "pages/page.html")]
pub struct PageTemplate {
    pub view: PageView,
}

pub async fn page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let row: Option<(String, String, String, bool, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT title, content_html, status, is_full_width, meta_title, meta_description
             FROM pages WHERE slug = ?",
        )
        .bind(&slug)
        .fetch_optional(&state.db)
        .await?;

    let (title, content_html, status, is_full_width, meta_title, meta_description) =
        row.ok_or_else(|| AppError::not_found("That page does not exist."))?;

    if status != "published" {
        return Err(AppError::not_found("That page does not exist."));
    }

    let mut base = context::build(&state, auth.0.as_ref(), &headers, &format!("/{slug}")).await?;
    base.page_title = meta_title.unwrap_or_else(|| format!("{title} · {}", base.site.title));
    base.meta_description = meta_description.unwrap_or(base.meta_description);

    let view = PageView {
        title,
        content_html,
        is_full_width,
        base,
    };

    Ok(crate::templates::Html(PageTemplate { view }).into_response())
}

// ---------------------------------------------------------------------------
// Resume
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "pages/resume.html")]
pub struct ResumeTemplate {
    pub view: ResumeView,
}

pub async fn resume(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/resume").await?;
    base.page_title = if base.lang == "fa" {
        format!("رزومه · {}", base.site.title)
    } else {
        format!("Résumé · {}", base.site.title)
    };

    let cv_file = base.site.cv_file.clone();
    let view = ResumeView {
        work: data::experiences(&state.db, "work", &base.lang).await?,
        education: data::experiences(&state.db, "education", &base.lang).await?,
        skills: data::skill_groups(&state.db, &base.lang).await?,
        stats: data::stats(&state.db, base.lang == "fa").await?,
        cv_file,
        base,
    };

    Ok(crate::templates::Html(ResumeTemplate { view }).into_response())
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "pages/search.html")]
pub struct SearchTemplate {
    pub view: SearchView,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

pub async fn search(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
    Query(query): Query<SearchQuery>,
) -> AppResult<Response> {
    let mut base = context::build(&state, auth.0.as_ref(), &headers, "/search").await?;
    let term = query.q.unwrap_or_default().trim().to_string();

    let mut posts: Vec<PostCard> = Vec::new();
    let mut projects: Vec<ProjectCard> = Vec::new();
    let mut pages: Vec<PageViewItem> = Vec::new();

    if !term.is_empty() {
        let pattern = format!("%{}%", term.to_lowercase());

        let summaries: Vec<crate::models::db::PostSummary> = sqlx::query_as(
            "SELECT p.id, p.slug, p.title, p.excerpt, p.cover_image, p.status, p.category_id,
                    c.name AS category_name, c.slug AS category_slug, u.display_name AS author_name,
                    p.is_featured, p.view_count, p.reading_time, p.published_at, p.updated_at
             FROM posts p
             LEFT JOIN categories c ON c.id = p.category_id
             LEFT JOIN users u ON u.id = p.author_id
             WHERE p.status = 'published'
               AND (LOWER(p.title) LIKE ? OR LOWER(p.excerpt) LIKE ? OR LOWER(p.content_md) LIKE ?)
             ORDER BY p.published_at DESC LIMIT 30",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(&state.db)
        .await?;
        posts = summaries.iter().map(PostCard::from_summary).collect();
        data::attach_tags(&state.db, &mut posts).await?;

        let project_rows = data::projects(&state.db, "active").await?;
        projects = project_rows
            .iter()
            .filter(|p| {
                p.title.to_lowercase().contains(&term.to_lowercase())
                    || p.summary.to_lowercase().contains(&term.to_lowercase())
            })
            .map(ProjectCard::from_model)
            .collect();

        let page_rows: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT title, slug, content_html FROM pages WHERE status = 'published'",
        )
        .fetch_all(&state.db)
        .await?;
        pages = page_rows
            .into_iter()
            .filter(|(_, _, html)| html.to_lowercase().contains(&term.to_lowercase()))
            .map(|(title, slug, html)| PageViewItem {
                title,
                url: format!("/{slug}"),
                excerpt: util::truncate(&crate::markdown::to_plain_text(&html), 140),
            })
            .collect();
    }

    base.page_title = if base.lang == "fa" {
        if term.is_empty() {
            format!("جستجو · {}", base.site.title)
        } else {
            format!("جستجو: {term} · {}", base.site.title)
        }
    } else if term.is_empty() {
        format!("Search · {}", base.site.title)
    } else {
        format!("Search: {term} · {}", base.site.title)
    };

    let total = (posts.len() + projects.len() + pages.len()) as i64;
    let view = SearchView {
        query: term,
        posts,
        projects,
        pages,
        total,
        base,
    };

    Ok(crate::templates::Html(SearchTemplate { view }).into_response())
}

// ---------------------------------------------------------------------------
// Contact
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "pages/contact.html")]
pub struct ContactTemplate {
    pub view: ContactView,
}

#[derive(Debug, Deserialize)]
pub struct ContactForm {
    pub name: String,
    pub email: String,
    pub subject: Option<String>,
    pub message: String,
    /// Honeypot — must stay empty.
    #[serde(default)]
    pub website: Option<String>,
    /// Timestamp used to reject submissions that were filled in too quickly.
    #[serde(default)]
    pub started_at: Option<i64>,
}

pub async fn contact_form(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
) -> AppResult<Response> {
    let base = context::build(&state, auth.0.as_ref(), &headers, "/contact").await?;
    let settings = state.settings.snapshot();

    let view = ContactView {
        success: false,
        message: String::new(),
        errors: Vec::new(),
        form_name: String::new(),
        form_email: String::new(),
        form_subject: String::new(),
        form_message: String::new(),
        socials: base.socials.clone(),
        base,
    };

    let _ = settings;
    Ok(crate::templates::Html(ContactTemplate { view }).into_response())
}

pub async fn contact_submit(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
    Form(form): Form<ContactForm>,
) -> AppResult<Response> {
    let base = context::build(&state, auth.0.as_ref(), &headers, "/contact").await?;
    let settings = state.settings.snapshot();

    let mut errors: Vec<String> = Vec::new();
    let name = form.name.trim().to_string();
    let email = form.email.trim().to_string();
    let subject = form.subject.unwrap_or_default().trim().to_string();
    let message = form.message.trim().to_string();

    let fa = base.lang == "fa";
    if name.is_empty() || name.chars().count() < 2 {
        errors.push(if fa { "لطفاً نام خود را بنویسید.".into() } else { "Please tell me your name.".into() });
    }
    if name.chars().count() > 120 {
        errors.push(if fa { "این نام کمی طولانی است.".into() } else { "That name is a little too long.".into() });
    }
    if !util::is_valid_email(&email) {
        errors.push(if fa { "لطفاً یک نشانی ایمیل معتبر وارد کنید.".into() } else { "Please enter a valid e-mail address.".into() });
    }
    if message.chars().count() < 10 {
        errors.push(if fa { "لطفاً دست‌کم یک یا دو جمله بنویسید.".into() } else { "Please write at least a sentence or two.".into() });
    }
    if message.chars().count() > 10_000 {
        errors.push(if fa { "این پیام بیش از حد طولانی است.".into() } else { "That message is too long.".into() });
    }

    // Honeypot: a filled-in hidden field means a bot.
    if form
        .website
        .as_deref()
        .map(|w| !w.is_empty())
        .unwrap_or(false)
    {
        tracing::warn!("contact form rejected by honeypot");
        return Ok(Redirect::to("/contact?sent=1").into_response());
    }

    // A form completed in under two seconds is almost certainly automated.
    if let Some(started_at) = form.started_at {
        let elapsed = chrono::Utc::now().timestamp() - started_at;
        if elapsed < 2 {
            errors.push(if fa { "کمی سریع ارسال شد — لطفاً دوباره تلاش کنید.".into() } else { "That was submitted a little too quickly — please try again.".into() });
        }
    }

    if !errors.is_empty() {
        let view = ContactView {
            success: false,
            message: if fa { "لطفاً خطاهای زیر را اصلاح کنید.".into() } else { "Please correct the errors below.".into() },
            errors,
            form_name: name,
            form_email: email,
            form_subject: subject,
            form_message: message,
            socials: base.socials.clone(),
            base,
        };
        return Ok(crate::templates::Html(ContactTemplate { view }).into_response());
    }

    let ip = util::client_ip(&headers);
    let agent = util::user_agent(&headers);

    sqlx::query(
        "INSERT INTO messages (name, email, subject, body, ip, user_agent, status)
         VALUES (?, ?, ?, ?, ?, ?, 'new')",
    )
    .bind(&name)
    .bind(&email)
    .bind(if subject.is_empty() {
        None
    } else {
        Some(&subject)
    })
    .bind(&message)
    .bind(&ip)
    .bind(&agent)
    .execute(&state.db)
    .await?;

    // Notify the owner; a failure here must not lose the message.
    let notify_to = if settings.contact_notify_email.is_empty() {
        settings.email.clone()
    } else {
        settings.contact_notify_email.clone()
    };
    if !notify_to.is_empty() {
        let subject_line = if subject.is_empty() {
            format!("Message from {name}")
        } else {
            subject.clone()
        };
        if let Err(err) = state
            .mailer
            .notify_contact(&notify_to, &name, &email, &subject_line, &message)
            .await
        {
            tracing::warn!(error = %err, "failed to send contact notification");
        }
    }

    crate::audit(
        &state,
        None,
        "contact.received",
        Some("message"),
        None,
        Some(&name),
    )
    .await;

    Ok(Redirect::to("/contact?sent=1").into_response())
}

// ---------------------------------------------------------------------------
// Newsletter
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "pages/newsletter_result.html")]
pub struct NewsletterResultTemplate {
    pub view: NewsletterView,
}

#[derive(Debug, Deserialize)]
pub struct SubscribeForm {
    pub email: String,
    pub name: Option<String>,
    /// Honeypot.
    #[serde(default)]
    pub website: Option<String>,
}

pub async fn subscribe(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    auth: crate::extract::MaybeAuth,
    Form(form): Form<SubscribeForm>,
) -> AppResult<Response> {
    let base = context::build(&state, auth.0.as_ref(), &headers, "/").await?;
    let settings = state.settings.snapshot();

    let email = form.email.trim().to_lowercase();
    let success;
    let message;

    let fa = base.lang == "fa";
    if form
        .website
        .as_deref()
        .map(|w| !w.is_empty())
        .unwrap_or(false)
    {
        success = true;
        message = if fa { "عضو خبرنامه شدید. متشکریم!".into() } else { "You are subscribed. Thank you!".into() };
    } else if !util::is_valid_email(&email) {
        success = false;
        message = if fa { "لطفاً یک نشانی ایمیل معتبر وارد کنید.".into() } else { "Please enter a valid e-mail address.".into() };
    } else {
        let token = uuid::Uuid::new_v4().to_string();
        let result = sqlx::query(
            "INSERT INTO subscribers (email, name, status, token, source, confirmed_at)
             VALUES (?, ?, 'active', ?, 'website', ?)
             ON CONFLICT(email) DO UPDATE SET status = 'active', token = excluded.token",
        )
        .bind(&email)
        .bind(
            form.name
                .as_deref()
                .map(|n| n.trim())
                .filter(|n| !n.is_empty()),
        )
        .bind(&token)
        .bind(chrono::Utc::now())
        .execute(&state.db)
        .await;

        match result {
            Ok(_) => {
                success = true;
                message = if fa { "عضو خبرنامه شدید. متشکریم!".into() } else { "You are subscribed. Thank you!".into() };
                crate::audit(
                    &state,
                    None,
                    "newsletter.subscribe",
                    Some("subscriber"),
                    None,
                    Some(&email),
                )
                .await;
            }
            Err(err) => {
                tracing::warn!(error = %err, "failed to subscribe");
                success = false;
                message = if fa { "خطایی رخ داد. لطفاً بعداً دوباره تلاش کنید.".into() } else { "Something went wrong. Please try again later.".into() };
            }
        }
    }

    let view = NewsletterView {
        heading: if !fa && !settings.newsletter_heading_en.is_empty() {
            settings.newsletter_heading_en.clone()
        } else {
            settings.newsletter_heading.clone()
        },
        subheading: if !fa && !settings.newsletter_subheading_en.is_empty() {
            settings.newsletter_subheading_en.clone()
        } else {
            settings.newsletter_subheading.clone()
        },
        success,
        message,
        base,
    };

    Ok(crate::templates::Html(NewsletterResultTemplate { view }).into_response())
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// The optional `?next=` query used by the language switcher.
#[derive(Debug, Deserialize)]
pub struct LangQuery {
    pub next: Option<String>,
}

/// Set the preferred language via a cookie, then redirect back to the page the
/// visitor came from (or home).
pub async fn set_language(
    Path(locale): Path<String>,
    Query(query): Query<LangQuery>,
) -> Response {
    let locale = if locale == "en" { "en" } else { "fa" };
    let cookie = crate::auth::session::language_cookie(locale);

    let next = query.next.unwrap_or_else(|| "/".into());
    // Only allow relative paths to avoid open redirects.
    let next = if next.starts_with('/') && !next.starts_with("//") {
        next
    } else {
        "/".into()
    };

    let mut response = Redirect::to(&next).into_response();
    if let Ok(value) = axum::http::HeaderValue::from_str(&cookie.to_string()) {
        response.headers_mut().insert(
            axum::http::header::SET_COOKIE,
            value,
        );
    }
    response
}

/// Return a 404 response rendered by the custom error page.
pub async fn not_found() -> Response {
    let status = StatusCode::NOT_FOUND;
    crate::templates::error_page(
        status,
        "The page you were looking for has moved or never existed.",
    )
    .into_response()
}
