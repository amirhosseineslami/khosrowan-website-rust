//! Reusable data loaders used by both the public site and the admin panel.

use chrono::Datelike;
use sqlx::SqlitePool;

use crate::error::AppResult;
use crate::models::db::*;
use crate::util;
use crate::views::*;

/// Load published post summaries.
pub async fn published_posts(
    pool: &SqlitePool,
    limit: Option<i64>,
    offset: Option<i64>,
) -> AppResult<Vec<PostSummary>> {
    let rows = sqlx::query_as::<_, PostSummary>(
        "SELECT p.id, p.slug, p.title, p.excerpt, p.cover_image, p.status, p.category_id,
                c.name AS category_name, c.slug AS category_slug, u.display_name AS author_name,
                p.is_featured, p.view_count, p.reading_time, p.published_at, p.updated_at
         FROM posts p
         LEFT JOIN categories c ON c.id = p.category_id
         LEFT JOIN users u ON u.id = p.author_id
         WHERE p.status = 'published' AND p.published_at <= ?
         ORDER BY p.is_featured DESC, p.published_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(chrono::Utc::now())
    .bind(limit.unwrap_or(i64::MAX))
    .bind(offset.unwrap_or(0))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Count published posts.
pub async fn count_published_posts(pool: &SqlitePool) -> AppResult<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts WHERE status = 'published' AND published_at <= ?",
    )
    .bind(chrono::Utc::now())
    .fetch_one(pool)
    .await?;
    Ok(count)
}

/// A join row between a post and a tag.
#[derive(sqlx::FromRow)]
struct PostTagRow {
    post_id: i64,
    name: String,
    slug: String,
    color: Option<String>,
    post_count: i64,
}

/// Load the tags attached to a set of posts.
pub async fn tags_for_posts(pool: &SqlitePool, post_ids: &[i64]) -> AppResult<Vec<(i64, TagView)>> {
    if post_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = post_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT pt.post_id, t.name, t.slug, t.color, t.post_count
         FROM post_tags pt JOIN tags t ON t.id = pt.tag_id
         WHERE pt.post_id IN ({placeholders})
         ORDER BY t.name ASC"
    );

    let mut query = sqlx::query_as::<_, PostTagRow>(&sql);
    for id in post_ids {
        query = query.bind(*id);
    }
    let rows: Vec<PostTagRow> = query.fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.post_id,
                TagView {
                    name: row.name,
                    slug: row.slug,
                    color: row.color.unwrap_or_else(|| "#6366f1".to_string()),
                    count: row.post_count,
                },
            )
        })
        .collect())
}

/// Attach tags to a list of post cards.
pub async fn attach_tags(pool: &SqlitePool, cards: &mut [PostCard]) -> AppResult<()> {
    if cards.is_empty() {
        return Ok(());
    }
    let ids: Vec<i64> = cards.iter().map(|c| c.id).collect();
    let mapping = tags_for_posts(pool, &ids).await?;

    for card in cards.iter_mut() {
        card.tags = mapping
            .iter()
            .filter(|(post_id, _)| *post_id == card.id)
            .map(|(_, tag)| tag.clone())
            .collect();
    }
    Ok(())
}

/// Load published posts as cards, with tags attached.
pub async fn post_cards(
    pool: &SqlitePool,
    limit: Option<i64>,
    offset: Option<i64>,
) -> AppResult<Vec<PostCard>> {
    let summaries = published_posts(pool, limit, offset).await?;
    let mut cards: Vec<PostCard> = summaries.iter().map(PostCard::from_summary).collect();
    attach_tags(pool, &mut cards).await?;
    Ok(cards)
}

/// Overlay translated title/excerpt onto cards for the given locale.
pub async fn apply_post_card_translations(
    pool: &SqlitePool,
    cards: &mut [PostCard],
    locale: &str,
) -> AppResult<()> {
    if locale == "fa" || cards.is_empty() {
        return Ok(());
    }
    let ids: Vec<i64> = cards.iter().map(|c| c.id).collect();
    let trans = post_translations(pool, &ids, locale).await?;

    // Category and tag name translations for the active locale.
    let cat_trans: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.slug, ct.name FROM category_translations ct
         JOIN categories c ON c.id = ct.category_id WHERE ct.locale = ?",
    )
    .bind(locale)
    .fetch_all(pool)
    .await?;
    let tag_trans: Vec<(String, String)> = sqlx::query_as(
        "SELECT t.slug, tt.name FROM tag_translations tt
         JOIN tags t ON t.id = tt.tag_id WHERE tt.locale = ?",
    )
    .bind(locale)
    .fetch_all(pool)
    .await?;

    // The author byline shows the brand, which is locale-aware.
    let brand: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'site.title_en'",
    )
    .fetch_optional(pool)
    .await?;
    let brand = brand.filter(|v| !v.is_empty()).unwrap_or_else(|| "Khosrowan".into());

    for card in cards.iter_mut() {
        if let Some(t) = trans.iter().find(|t| t.post_id == card.id) {
            card.title = t.title.clone();
            card.excerpt = t.excerpt.clone();
        }
        if let Some((_, name)) = cat_trans.iter().find(|(slug, _)| *slug == card.category_slug) {
            card.category_name = name.clone();
        }
        for tag in &mut card.tags {
            if let Some((_, name)) = tag_trans.iter().find(|(slug, _)| *slug == tag.slug) {
                tag.name = name.clone();
            }
        }
        card.author = brand.clone();
        card.author_initials = util::initials(&brand);
    }
    Ok(())
}

/// Load the translated variant of a post for a locale, if it exists.
pub async fn post_translation(
    pool: &SqlitePool,
    post_id: i64,
    locale: &str,
) -> sqlx::Result<Option<crate::models::db::PostTranslation>> {
    sqlx::query_as::<_, crate::models::db::PostTranslation>(
        "SELECT * FROM post_translations WHERE post_id = ? AND locale = ?",
    )
    .bind(post_id)
    .bind(locale)
    .fetch_optional(pool)
    .await
}

/// Load translated variants for a set of posts.
pub async fn post_translations(
    pool: &SqlitePool,
    post_ids: &[i64],
    locale: &str,
) -> sqlx::Result<Vec<crate::models::db::PostTranslation>> {
    if post_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = post_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT * FROM post_translations WHERE locale = ? AND post_id IN ({placeholders})"
    );
    let mut query = sqlx::query_as::<_, crate::models::db::PostTranslation>(&sql).bind(locale);
    for id in post_ids {
        query = query.bind(*id);
    }
    query.fetch_all(pool).await
}

/// Load the translated variant of a project for a locale, if it exists.
pub async fn project_translation(
    pool: &SqlitePool,
    project_id: i64,
    locale: &str,
) -> sqlx::Result<Option<crate::models::db::ProjectTranslation>> {
    sqlx::query_as::<_, crate::models::db::ProjectTranslation>(
        "SELECT * FROM project_translations WHERE project_id = ? AND locale = ?",
    )
    .bind(project_id)
    .bind(locale)
    .fetch_optional(pool)
    .await
}

/// Load the translated variant of a page for a locale, if it exists.
pub async fn page_translation(
    pool: &SqlitePool,
    page_id: i64,
    locale: &str,
) -> sqlx::Result<Option<crate::models::db::PageTranslation>> {
    sqlx::query_as::<_, crate::models::db::PageTranslation>(
        "SELECT * FROM page_translations WHERE page_id = ? AND locale = ?",
    )
    .bind(page_id)
    .bind(locale)
    .fetch_optional(pool)
    .await
}

/// Load projects.
pub async fn projects(pool: &SqlitePool, status: &str) -> AppResult<Vec<Project>> {
    let rows = sqlx::query_as::<_, Project>(
        "SELECT * FROM projects WHERE status = ? ORDER BY sort_order ASC, created_at DESC",
    )
    .bind(status)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Load project cards with their index numbers.
pub async fn project_cards(pool: &SqlitePool, status: &str) -> AppResult<Vec<ProjectCard>> {
    let rows = projects(pool, status).await?;
    Ok(rows
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut card = ProjectCard::from_model(p);
            card.index = format!("{:02}", i + 1);
            card
        })
        .collect())
}

/// Overlay translated title/summary onto project cards for the given locale.
pub async fn apply_project_card_translations(
    pool: &SqlitePool,
    cards: &mut [ProjectCard],
    locale: &str,
) -> AppResult<()> {
    if locale == "fa" || cards.is_empty() {
        return Ok(());
    }
    let ids: Vec<i64> = cards.iter().map(|c| c.id).collect();
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT * FROM project_translations WHERE locale = ? AND project_id IN ({placeholders})"
    );
    let mut query = sqlx::query_as::<_, crate::models::db::ProjectTranslation>(&sql).bind(locale);
    for id in &ids {
        query = query.bind(*id);
    }
    let trans = query.fetch_all(pool).await?;
    for card in cards.iter_mut() {
        if let Some(t) = trans.iter().find(|t| t.project_id == card.id) {
            card.title = t.title.clone();
            card.summary = t.summary.clone();
        }
    }
    Ok(())
}

/// Load a single project by slug.
pub async fn project_by_slug(pool: &SqlitePool, slug: &str) -> AppResult<Option<Project>> {
    let row = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE slug = ?")
        .bind(slug)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

/// Load a single post by slug (any status).
pub async fn post_by_slug(pool: &SqlitePool, slug: &str) -> AppResult<Option<Post>> {
    let row = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE slug = ?")
        .bind(slug)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

/// Load skills grouped by category, preferring the active locale's translation.
pub async fn skill_groups(pool: &SqlitePool, locale: &str) -> AppResult<Vec<SkillGroup>> {
    let rows: Vec<Skill> = sqlx::query_as("SELECT * FROM skills ORDER BY sort_order ASC, name ASC")
        .fetch_all(pool)
        .await?;

    let is_fa = locale == "fa";
    let trans: Vec<crate::models::db::SkillTranslation> = if is_fa {
        Vec::new()
    } else {
        sqlx::query_as("SELECT * FROM skill_translations WHERE locale = ?")
            .bind(locale)
            .fetch_all(pool)
            .await?
    };

    let mut groups: Vec<SkillGroup> = Vec::new();
    for skill in rows {
        let t = trans.iter().find(|t| t.skill_id == skill.id);
        let name = t.map(|t| t.name.clone()).unwrap_or_else(|| skill.name.clone());
        let category = t.map(|t| t.category.clone()).unwrap_or_else(|| skill.category.clone());
        let label = category_label(&category, is_fa);
        let item = SkillView {
            name,
            level: skill.level,
            years: skill
                .years
                .map(|y| {
                    let years = format!("{y:.0}");
                    if is_fa {
                        format!("{years} سال")
                    } else {
                        format!("{years} yr{}", if y == 1.0 { "" } else { "s" })
                    }
                })
                .unwrap_or_default(),
        };

        match groups.iter_mut().find(|g| g.category == category) {
            Some(group) => group.items.push(item),
            None => groups.push(SkillGroup {
                category,
                label,
                items: vec![item],
            }),
        }
    }
    Ok(groups)
}

/// Human readable label for a skill category.
pub fn category_label(category: &str, fa: bool) -> String {
    if fa {
        return match category {
            "languages" => "زبان‌ها".into(),
            "backend" => "بک‌اند".into(),
            "data" => "داده و ذخیره‌سازی".into(),
            "ops" => "زیرساخت".into(),
            "frontend" => "فرانت‌اند".into(),
            "other" => "سایر".into(),
            other => other.to_string(),
        };
    }
    match category {
        "languages" => "Languages".into(),
        "backend" => "Backend".into(),
        "data" => "Data & Storage".into(),
        "ops" => "Infrastructure".into(),
        "frontend" => "Frontend".into(),
        "other" => "Other".into(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

/// Load experiences split by kind, preferring the active locale's translation.
pub async fn experiences(
    pool: &SqlitePool,
    kind: &str,
    locale: &str,
) -> AppResult<Vec<ExperienceView>> {
    let rows: Vec<Experience> =
        sqlx::query_as("SELECT * FROM experiences WHERE kind = ? ORDER BY sort_order ASC, id ASC")
            .bind(kind)
            .fetch_all(pool)
            .await?;

    let is_fa = locale == "fa";
    let trans: Vec<crate::models::db::ExperienceTranslation> = if is_fa {
        Vec::new()
    } else {
        sqlx::query_as(
            "SELECT * FROM experience_translations WHERE locale = ? AND experience_id IN (
                 SELECT id FROM experiences WHERE kind = ?
             )",
        )
        .bind(locale)
        .bind(kind)
        .fetch_all(pool)
        .await?
    };

    Ok(rows
        .into_iter()
        .map(|row| {
            let t = trans.iter().find(|t| t.experience_id == row.id);
            let period = match (&row.start_date, &row.end_date, row.is_current) {
                (Some(start), Some(end), _) => format!("{start} — {end}"),
                (Some(start), None, true) => {
                    if is_fa {
                        format!("{start} — اکنون")
                    } else {
                        format!("{start} — Present")
                    }
                }
                (Some(start), None, false) => start.clone(),
                (None, Some(end), _) => end.clone(),
                (None, None, _) => String::new(),
            };
            ExperienceView {
                title: t.map(|t| t.title.clone()).unwrap_or(row.title),
                company: t.map(|t| t.company.clone()).unwrap_or(row.company),
                location: t
                    .and_then(|t| t.location.clone())
                    .or(row.location)
                    .unwrap_or_default(),
                period,
                is_current: row.is_current,
                description: t.map(|t| t.description.clone()).unwrap_or(row.description),
                kind: row.kind,
            }
        })
        .collect())
}

/// Load services, preferring the active locale's translation.
pub async fn services(pool: &SqlitePool, locale: &str) -> AppResult<Vec<ServiceView>> {
    let rows: Vec<Service> =
        sqlx::query_as("SELECT * FROM services ORDER BY sort_order ASC, id ASC")
            .fetch_all(pool)
            .await?;

    let is_fa = locale == "fa";
    let trans: Vec<crate::models::db::ServiceTranslation> = if is_fa {
        Vec::new()
    } else {
        sqlx::query_as("SELECT * FROM service_translations WHERE locale = ?")
            .bind(locale)
            .fetch_all(pool)
            .await?
    };

    Ok(rows
        .into_iter()
        .map(|row| {
            let t = trans.iter().find(|t| t.service_id == row.id);
            ServiceView {
                title: t.map(|t| t.title.clone()).unwrap_or(row.title),
                description: t.map(|t| t.description.clone()).unwrap_or(row.description),
                icon: row.icon.unwrap_or_else(|| "sparkles".into()),
                price_from: t
                    .and_then(|t| t.price_from.clone())
                    .or(row.price_from)
                    .unwrap_or_default(),
                features: t
                    .and_then(|t| t.features.clone())
                    .or(row.features)
                    .as_deref()
                    .and_then(|f| serde_json::from_str::<Vec<String>>(f).ok())
                    .unwrap_or_default(),
            }
        })
        .collect())
}

/// Load testimonials.
pub async fn testimonials(
    pool: &SqlitePool,
    only_featured: bool,
) -> AppResult<Vec<TestimonialView>> {
    let sql = if only_featured {
        "SELECT * FROM testimonials WHERE is_featured = 1 ORDER BY sort_order ASC"
    } else {
        "SELECT * FROM testimonials ORDER BY sort_order ASC"
    };
    let rows: Vec<Testimonial> = sqlx::query_as(sql).fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .map(|row| TestimonialView {
            name: row.name.clone(),
            role: row.role.unwrap_or_default(),
            company: row.company.unwrap_or_default(),
            body: row.body,
            initials: util::initials(&row.name),
            rating: row.rating,
            avatar: row.avatar,
        })
        .collect())
}

/// Load awards.
pub async fn awards(pool: &SqlitePool) -> AppResult<Vec<AwardView>> {
    let rows: Vec<Award> = sqlx::query_as("SELECT * FROM awards ORDER BY sort_order ASC, id ASC")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let has_link = row.link.is_some();
            AwardView {
                title: row.title,
                issuer: row.issuer.unwrap_or_default(),
                awarded_on: row.awarded_on.unwrap_or_default(),
                description: row.description.unwrap_or_default(),
                link: row.link.unwrap_or_default(),
                has_link,
            }
        })
        .collect())
}

/// Load talks.
pub async fn talks(pool: &SqlitePool) -> AppResult<Vec<TalkView>> {
    let rows: Vec<Talk> = sqlx::query_as("SELECT * FROM talks ORDER BY sort_order ASC, id ASC")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let has_video = row.video_url.is_some();
            let has_slides = row.slides_url.is_some();
            TalkView {
                title: row.title,
                event: row.event.unwrap_or_default(),
                date: row.date.unwrap_or_default(),
                location: row.location.unwrap_or_default(),
                video_url: row.video_url.unwrap_or_default(),
                slides_url: row.slides_url.unwrap_or_default(),
                has_video,
                has_slides,
                summary: row.summary.unwrap_or_default(),
            }
        })
        .collect())
}

/// Load publications.
pub async fn publications(pool: &SqlitePool) -> AppResult<Vec<PublicationView>> {
    let rows: Vec<Publication> =
        sqlx::query_as("SELECT * FROM publications ORDER BY sort_order ASC, id ASC")
            .fetch_all(pool)
            .await?;

    Ok(rows
        .into_iter()
        .map(|row| PublicationView {
            title: row.title,
            publisher: row.publisher.unwrap_or_default(),
            date: row.date.unwrap_or_default(),
            url: row.url.unwrap_or_default(),
            summary: row.summary.unwrap_or_default(),
        })
        .collect())
}

/// Load gallery items.
pub async fn gallery(pool: &SqlitePool) -> AppResult<Vec<GalleryViewItem>> {
    let rows: Vec<GalleryItem> =
        sqlx::query_as("SELECT * FROM gallery ORDER BY sort_order ASC, id ASC")
            .fetch_all(pool)
            .await?;

    Ok(rows
        .into_iter()
        .map(|row| GalleryViewItem {
            title: row.title.unwrap_or_default(),
            image: row.image,
            caption: row.caption.unwrap_or_default(),
            category: row.category.unwrap_or_default(),
        })
        .collect())
}

/// Load categories with their post counts, preferring the active locale.
pub async fn categories(pool: &SqlitePool, locale: &str) -> AppResult<Vec<CategoryView>> {
    let rows: Vec<Category> = sqlx::query_as("SELECT * FROM categories ORDER BY name ASC")
        .fetch_all(pool)
        .await?;
    let trans: Vec<crate::models::db::CategoryTranslation> = if locale == "fa" {
        Vec::new()
    } else {
        sqlx::query_as("SELECT * FROM category_translations WHERE locale = ?")
            .bind(locale)
            .fetch_all(pool)
            .await?
    };
    Ok(rows
        .into_iter()
        .map(|row| {
            let t = trans.iter().find(|t| t.category_id == row.id);
            CategoryView {
                name: t.map(|t| t.name.clone()).unwrap_or(row.name),
                slug: row.slug,
                description: t
                    .and_then(|t| t.description.clone())
                    .or(row.description)
                    .unwrap_or_default(),
                count: row.post_count,
            }
        })
        .collect())
}

/// Load tags with their post counts, preferring the active locale.
pub async fn tags(pool: &SqlitePool, locale: &str) -> AppResult<Vec<TagView>> {
    let rows: Vec<Tag> = sqlx::query_as("SELECT * FROM tags ORDER BY name ASC")
        .fetch_all(pool)
        .await?;
    let trans: Vec<crate::models::db::TagTranslation> = if locale == "fa" {
        Vec::new()
    } else {
        sqlx::query_as("SELECT * FROM tag_translations WHERE locale = ?")
            .bind(locale)
            .fetch_all(pool)
            .await?
    };
    Ok(rows
        .into_iter()
        .map(|row| {
            let t = trans.iter().find(|t| t.tag_id == row.id);
            TagView {
                name: t.map(|t| t.name.clone()).unwrap_or(row.name),
                slug: row.slug,
                color: row.color.unwrap_or_else(|| "#6366f1".to_string()),
                count: row.post_count,
            }
        })
        .collect())
}

pub fn stats_meta(label: &'static str, fa: bool) -> (&'static str, &'static str) {
    match (label, fa) {
        ("Years of experience", true) => ("سال تجربه", "طراحی و اجرای سیستم‌های واقعی"),
        ("Projects shipped", true) => ("پروژه‌های اجراشده", "از نمونه‌اولیه تا پلتفرم"),
        ("Articles published", true) => ("مقاله منتشرشده", "نوشته‌های فنی بلند"),
        ("Teams worked with", true) => ("همکاری‌ها", "پروژه‌ها و تیم‌های مختلف"),
        ("Years of experience", false) => ("Years of experience", "Building and running production systems"),
        ("Projects shipped", false) => ("Projects shipped", "From prototypes to platforms"),
        ("Articles published", false) => ("Articles published", "Long-form writing on engineering"),
        ("Teams worked with", false) => ("Teams worked with", "Startups to Fortune 500"),
        _ => (label, ""),
    }
}

/// Load the site statistics shown across the site.
pub async fn stats(pool: &SqlitePool, fa: bool) -> AppResult<Vec<StatView>> {
    let posts = count_published_posts(pool).await?;
    let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE status = 'active'")
        .fetch_one(pool)
        .await?;
    let years: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MIN(CAST(substr(start_date,1,4) AS INTEGER)), 0) FROM experiences WHERE kind = 'work'",
    )
    .fetch_one(pool)
    .await?;
    let current_year = chrono::Utc::now().year() as i64;
    let experience_years = if years > 0 {
        current_year - years + 1
    } else {
        0
    };

    let clients: i64 =
        sqlx::query_scalar("SELECT COUNT(DISTINCT company) FROM experiences WHERE kind = 'work'")
            .fetch_one(pool)
            .await?;

    let meta_years = stats_meta("Years of experience", fa);
    let meta_projects = stats_meta("Projects shipped", fa);
    let meta_posts = stats_meta("Articles published", fa);
    let meta_clients = stats_meta("Teams worked with", fa);
    Ok(vec![
        StatView {
            value: format!("{experience_years}+"),
            label: meta_years.0.to_string(),
            hint: meta_years.1.to_string(),
        },
        StatView {
            value: format!("{projects}"),
            label: meta_projects.0.to_string(),
            hint: meta_projects.1.to_string(),
        },
        StatView {
            value: format!("{posts}"),
            label: meta_posts.0.to_string(),
            hint: meta_posts.1.to_string(),
        },
        StatView {
            value: format!("{clients}"),
            label: meta_clients.0.to_string(),
            hint: meta_clients.1.to_string(),
        },
    ])
}
