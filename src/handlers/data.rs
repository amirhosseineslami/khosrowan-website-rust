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

/// Load skills grouped by category.
pub async fn skill_groups(pool: &SqlitePool) -> AppResult<Vec<SkillGroup>> {
    let rows: Vec<Skill> = sqlx::query_as("SELECT * FROM skills ORDER BY sort_order ASC, name ASC")
        .fetch_all(pool)
        .await?;

    let mut groups: Vec<SkillGroup> = Vec::new();
    for skill in rows {
        let label = category_label(&skill.category);
        let item = SkillView {
            name: skill.name.clone(),
            level: skill.level,
            years: skill
                .years
                .map(|y| format!("{y:.0} yr{}", if y == 1.0 { "" } else { "s" }))
                .unwrap_or_default(),
        };

        match groups.iter_mut().find(|g| g.category == skill.category) {
            Some(group) => group.items.push(item),
            None => groups.push(SkillGroup {
                category: skill.category.clone(),
                label,
                items: vec![item],
            }),
        }
    }
    Ok(groups)
}

/// Human readable label for a skill category.
pub fn category_label(category: &str) -> String {
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

/// Load experiences split by kind.
pub async fn experiences(pool: &SqlitePool, kind: &str) -> AppResult<Vec<ExperienceView>> {
    let rows: Vec<Experience> =
        sqlx::query_as("SELECT * FROM experiences WHERE kind = ? ORDER BY sort_order ASC, id ASC")
            .bind(kind)
            .fetch_all(pool)
            .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let period = match (&row.start_date, &row.end_date, row.is_current) {
                (Some(start), Some(end), _) => format!("{start} — {end}"),
                (Some(start), None, true) => format!("{start} — Present"),
                (Some(start), None, false) => start.clone(),
                (None, Some(end), _) => end.clone(),
                (None, None, _) => String::new(),
            };
            ExperienceView {
                title: row.title,
                company: row.company,
                location: row.location.unwrap_or_default(),
                period,
                is_current: row.is_current,
                description: row.description,
                kind: row.kind,
            }
        })
        .collect())
}

/// Load services.
pub async fn services(pool: &SqlitePool) -> AppResult<Vec<ServiceView>> {
    let rows: Vec<Service> =
        sqlx::query_as("SELECT * FROM services ORDER BY sort_order ASC, id ASC")
            .fetch_all(pool)
            .await?;

    Ok(rows
        .into_iter()
        .map(|row| ServiceView {
            title: row.title,
            description: row.description,
            icon: row.icon.unwrap_or_else(|| "sparkles".into()),
            price_from: row.price_from.unwrap_or_default(),
            features: row
                .features
                .as_deref()
                .and_then(|f| serde_json::from_str::<Vec<String>>(f).ok())
                .unwrap_or_default(),
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

/// Load categories with their post counts.
pub async fn categories(pool: &SqlitePool) -> AppResult<Vec<CategoryView>> {
    let rows: Vec<Category> = sqlx::query_as("SELECT * FROM categories ORDER BY name ASC")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| CategoryView {
            name: row.name,
            slug: row.slug,
            description: row.description.unwrap_or_default(),
            count: row.post_count,
        })
        .collect())
}

/// Load tags with their post counts.
pub async fn tags(pool: &SqlitePool) -> AppResult<Vec<TagView>> {
    let rows: Vec<Tag> = sqlx::query_as("SELECT * FROM tags ORDER BY name ASC")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| TagView {
            name: row.name,
            slug: row.slug,
            color: row.color.unwrap_or_else(|| "#6366f1".to_string()),
            count: row.post_count,
        })
        .collect())
}

/// Load the site statistics shown across the site.
pub async fn stats(pool: &SqlitePool) -> AppResult<Vec<StatView>> {
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

    Ok(vec![
        StatView {
            value: format!("{experience_years}+"),
            label: "Years of experience".into(),
            hint: "Building and running production systems".into(),
        },
        StatView {
            value: format!("{projects}"),
            label: "Projects shipped".into(),
            hint: "From prototypes to platforms".into(),
        },
        StatView {
            value: format!("{posts}"),
            label: "Articles published".into(),
            hint: "Long-form writing on engineering".into(),
        },
        StatView {
            value: format!("{clients}"),
            label: "Teams worked with".into(),
            hint: "Startups to Fortune 500".into(),
        },
    ])
}
