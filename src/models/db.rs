//! Database row models.
//!
//! Every struct in this module maps 1:1 onto a table and derives
//! `sqlx::FromRow` so it can be fetched with runtime-checked queries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A registered account. Only `Owner` and `Editor` can reach the admin panel.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    /// `owner` | `editor`
    pub role: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub bio: Option<String>,
    pub totp_secret: Option<String>,
    pub totp_enabled: bool,
    pub is_active: bool,
    pub failed_logins: i64,
    pub locked_until: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: i64,
    pub csrf_token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Post {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub content_md: String,
    pub content_html: String,
    pub cover_image: Option<String>,
    /// `draft` | `published` | `scheduled` | `archived`
    pub status: String,
    pub category_id: Option<i64>,
    pub author_id: i64,
    pub is_featured: bool,
    pub allow_comments: bool,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image: Option<String>,
    pub view_count: i64,
    pub reading_time: i64,
    pub published_at: Option<DateTime<Utc>>,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PostSummary {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub cover_image: Option<String>,
    pub status: String,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    pub category_slug: Option<String>,
    pub author_name: Option<String>,
    pub is_featured: bool,
    pub view_count: i64,
    pub reading_time: i64,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

/// A translated variant of a post. Falls back to the base row when absent.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PostTranslation {
    pub post_id: i64,
    pub locale: String,
    pub title: String,
    pub excerpt: String,
    pub content_md: String,
    pub content_html: String,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
}

/// A translated variant of a page.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PageTranslation {
    pub page_id: i64,
    pub locale: String,
    pub title: String,
    pub content_md: String,
    pub content_html: String,
}

/// A translated variant of a project.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectTranslation {
    pub project_id: i64,
    pub locale: String,
    pub title: String,
    pub summary: String,
    pub description_md: String,
    pub description_html: String,
}

/// A translated variant of an experience entry.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExperienceTranslation {
    pub experience_id: i64,
    pub locale: String,
    pub title: String,
    pub company: String,
    pub location: Option<String>,
    pub description: String,
}

/// A translated variant of a service entry.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServiceTranslation {
    pub service_id: i64,
    pub locale: String,
    pub title: String,
    pub description: String,
    pub features: Option<String>,
    pub price_from: Option<String>,
}

/// A translated variant of a skill entry.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SkillTranslation {
    pub skill_id: i64,
    pub locale: String,
    pub name: String,
    pub category: String,
}

/// A translated variant of a category.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CategoryTranslation {
    pub category_id: i64,
    pub locale: String,
    pub name: String,
    pub description: Option<String>,
}

/// A translated variant of a tag.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TagTranslation {
    pub tag_id: i64,
    pub locale: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub post_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub post_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub description_md: String,
    pub description_html: String,
    pub cover_image: Option<String>,
    pub gallery: Option<String>,
    pub repo_url: Option<String>,
    pub live_url: Option<String>,
    /// `active` | `archived` | `draft`
    pub status: String,
    pub client: Option<String>,
    pub is_featured: bool,
    pub sort_order: i64,
    pub tech_stack: Option<String>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Page {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub content_md: String,
    pub content_html: String,
    pub status: String,
    pub is_full_width: bool,
    pub show_in_nav: bool,
    pub nav_order: i64,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Comment {
    pub id: i64,
    pub post_id: i64,
    pub parent_id: Option<i64>,
    pub post_title: Option<String>,
    pub post_slug: Option<String>,
    pub author_name: String,
    pub author_email: Option<String>,
    pub author_website: Option<String>,
    pub body: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    /// `pending` | `approved` | `spam` | `trash`
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Message {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub subject: Option<String>,
    pub body: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    /// `new` | `read` | `archived` | `spam`
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Subscriber {
    pub id: i64,
    pub email: String,
    pub name: Option<String>,
    /// `active` | `unsubscribed` | `pending`
    pub status: String,
    pub token: String,
    pub source: Option<String>,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MediaItem {
    pub id: i64,
    pub filename: String,
    pub stored_name: String,
    pub path: String,
    pub mime: String,
    pub size: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub alt_text: Option<String>,
    pub uploaded_by: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Skill {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub level: i64,
    pub years: Option<f64>,
    pub icon: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Experience {
    pub id: i64,
    pub title: String,
    pub company: String,
    pub location: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub is_current: bool,
    pub description: String,
    /// `work` | `education` | `volunteer`
    pub kind: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Service {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub icon: Option<String>,
    pub price_from: Option<String>,
    pub features: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Testimonial {
    pub id: i64,
    pub name: String,
    pub role: Option<String>,
    pub company: Option<String>,
    pub body: String,
    pub avatar: Option<String>,
    pub rating: i64,
    pub is_featured: bool,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Award {
    pub id: i64,
    pub title: String,
    pub issuer: Option<String>,
    pub awarded_on: Option<String>,
    pub description: Option<String>,
    pub link: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Talk {
    pub id: i64,
    pub title: String,
    pub event: Option<String>,
    pub date: Option<String>,
    pub location: Option<String>,
    pub video_url: Option<String>,
    pub slides_url: Option<String>,
    pub summary: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Publication {
    pub id: i64,
    pub title: String,
    pub publisher: Option<String>,
    pub date: Option<String>,
    pub url: Option<String>,
    pub summary: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GalleryItem {
    pub id: i64,
    pub title: Option<String>,
    pub image: String,
    pub caption: Option<String>,
    pub category: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SocialLink {
    pub id: i64,
    pub platform: String,
    pub url: String,
    pub label: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Redirect {
    pub id: i64,
    pub from_path: String,
    pub to_path: String,
    pub status_code: i64,
    pub hits: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditEntry {
    pub id: i64,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub action: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub details: Option<String>,
    pub ip: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PageView {
    pub id: i64,
    pub path: String,
    pub referrer: Option<String>,
    pub user_agent: Option<String>,
    pub country: Option<String>,
    pub day: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Campaign {
    pub id: i64,
    pub subject: String,
    pub body_html: String,
    pub status: String,
    pub recipient_count: i64,
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SettingRow {
    pub key: String,
    pub value: Option<String>,
    pub value_type: String,
    pub group_name: String,
    pub label: Option<String>,
    pub help_text: Option<String>,
    pub updated_at: DateTime<Utc>,
}
