//! View models. Templates only ever see these structs, never database rows.

use serde::Serialize;

use crate::util;

// ---------------------------------------------------------------------------
// Shared
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SiteInfo {
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub tagline: String,
    pub email: String,
    pub phone: String,
    pub location: String,
    pub available: bool,
    pub availability_note: String,
    pub logo_text: String,
    pub favicon: String,
    pub cv_file: String,
}

#[derive(Debug, Clone)]
pub struct NavItem {
    pub label: String,
    pub url: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct SocialView {
    pub platform: String,
    pub url: String,
    pub label: String,
    pub icon: String,
}

#[derive(Debug, Clone)]
pub struct UserView {
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub role: String,
    pub avatar: Option<String>,
    pub initials: String,
    pub is_owner: bool,
}

#[derive(Debug, Clone)]
pub struct FlashView {
    pub kind: String,
    pub message: String,
}

/// Data available on every page.
#[derive(Debug, Clone)]
pub struct BaseContext {
    pub site: SiteInfo,
    pub nav: Vec<NavItem>,
    pub socials: Vec<SocialView>,
    pub user: Option<UserView>,
    pub flash: FlashView,
    pub current_path: String,
    pub canonical_url: String,
    pub page_title: String,
    pub meta_description: String,
    pub og_image: String,
    pub og_type: String,
    pub twitter_site: String,
    pub year: i32,
    pub accent: String,
    pub accent2: String,
    pub default_theme: String,
    pub enable_dark_mode: bool,
    pub analytics_provider: String,
    pub plausible_domain: String,
    pub ga_id: String,
    pub analytics_head: String,
    pub analytics_footer: String,
    pub google_verification: String,
    pub bing_verification: String,
    pub base_url: String,
    pub rss_url: String,
    pub sitemap_url: String,
    pub csrf_token: String,
}

// ---------------------------------------------------------------------------
// Blog
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TagView {
    pub name: String,
    pub slug: String,
    pub color: String,
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct CategoryView {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub count: i64,
}

#[derive(Debug, Clone)]
pub struct PostCard {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub url: String,
    pub cover_image: String,
    pub has_cover: bool,
    pub category: String,
    pub category_slug: String,
    pub category_name: String,
    pub tags: Vec<TagView>,
    pub author: String,
    pub author_initials: String,
    pub published_at: String,
    pub published_iso: String,
    pub time_ago: String,
    pub reading_time: i64,
    pub view_count: i64,
    pub is_featured: bool,
}

#[derive(Debug, Clone)]
pub struct PostView {
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub content_html: String,
    pub cover_image: String,
    pub has_cover: bool,
    pub category: String,
    pub category_slug: String,
    pub category_name: String,
    pub tags: Vec<TagView>,
    pub author: String,
    pub author_initials: String,
    pub author_bio: String,
    pub published_at: String,
    pub published_iso: String,
    pub updated_at: String,
    pub reading_time: i64,
    pub word_count: usize,
    pub view_count: i64,
    pub allow_comments: bool,
    pub toc: Vec<TocItem>,
    pub comments: Vec<CommentView>,
    pub prev: Option<PostCard>,
    pub next: Option<PostCard>,
    pub related: Vec<PostCard>,
    pub share_url: String,
}

#[derive(Debug, Clone)]
pub struct TocItem {
    pub level: u32,
    pub text: String,
    pub anchor: String,
}

#[derive(Debug, Clone)]
pub struct CommentView {
    pub id: i64,
    pub author_name: String,
    pub author_initials: String,
    pub author_website: String,
    pub body: String,
    pub created_at: String,
    pub created_iso: String,
    pub time_ago: String,
    pub replies: Vec<CommentView>,
    pub depth: u32,
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ProjectCard {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub url: String,
    pub cover_image: String,
    pub has_cover: bool,
    pub repo_url: String,
    pub live_url: String,
    pub has_repo: bool,
    pub has_live: bool,
    pub is_featured: bool,
    pub tech: Vec<String>,
    pub index: String,
}

#[derive(Debug, Clone)]
pub struct ProjectView {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub description_html: String,
    pub cover_image: String,
    pub has_cover: bool,
    pub repo_url: String,
    pub live_url: String,
    pub client: String,
    pub tech: Vec<String>,
    pub tags: Vec<TagView>,
    pub created_at: String,
    pub updated_at: String,
    pub related: Vec<ProjectCard>,
    pub index: String,
}

// ---------------------------------------------------------------------------
// Profile sections
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SkillView {
    pub name: String,
    pub level: i64,
    pub years: String,
}

#[derive(Debug, Clone)]
pub struct SkillGroup {
    pub category: String,
    pub label: String,
    pub items: Vec<SkillView>,
}

#[derive(Debug, Clone)]
pub struct ExperienceView {
    pub title: String,
    pub company: String,
    pub location: String,
    pub period: String,
    pub is_current: bool,
    pub description: String,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct ServiceView {
    pub title: String,
    pub description: String,
    pub icon: String,
    pub price_from: String,
    pub features: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TestimonialView {
    pub name: String,
    pub role: String,
    pub company: String,
    pub body: String,
    pub initials: String,
    pub rating: i64,
    pub avatar: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AwardView {
    pub title: String,
    pub issuer: String,
    pub awarded_on: String,
    pub description: String,
    pub link: String,
    pub has_link: bool,
}

#[derive(Debug, Clone)]
pub struct TalkView {
    pub title: String,
    pub event: String,
    pub date: String,
    pub location: String,
    pub video_url: String,
    pub slides_url: String,
    pub has_video: bool,
    pub has_slides: bool,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct PublicationView {
    pub title: String,
    pub publisher: String,
    pub date: String,
    pub url: String,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct GalleryView {
    pub title: String,
    pub image: String,
    pub caption: String,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct StatView {
    pub value: String,
    pub label: String,
    pub hint: String,
}

// ---------------------------------------------------------------------------
// Admin
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AdminStats {
    pub posts_total: i64,
    pub posts_published: i64,
    pub posts_draft: i64,
    pub posts_scheduled: i64,
    pub projects_total: i64,
    pub pages_total: i64,
    pub comments_pending: i64,
    pub comments_total: i64,
    pub messages_new: i64,
    pub messages_total: i64,
    pub subscribers: i64,
    pub views_total: i64,
    pub views_week: i64,
    pub media_count: i64,
    pub users_count: i64,
    pub storage_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct AdminPostRow {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub status: String,
    pub status_label: String,
    pub status_class: String,
    pub category: String,
    pub views: i64,
    pub comments: i64,
    pub published_at: String,
    pub updated_at: String,
    pub is_featured: bool,
    pub scheduled_for: String,
}

#[derive(Debug, Clone)]
pub struct AdminProjectRow {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub status: String,
    pub status_class: String,
    pub tech: Vec<String>,
    pub is_featured: bool,
    pub sort_order: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct AdminPageRow {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub status: String,
    pub status_class: String,
    pub show_in_nav: bool,
    pub nav_order: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct AdminCommentRow {
    pub id: i64,
    pub post_title: String,
    pub post_slug: String,
    pub author_name: String,
    pub author_email: String,
    pub body: String,
    pub status: String,
    pub status_class: String,
    pub created_at: String,
    pub created_iso: String,
    pub ip: String,
}

#[derive(Debug, Clone)]
pub struct AdminMessageRow {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub status_class: String,
    pub created_at: String,
    pub created_iso: String,
    pub ip: String,
}

#[derive(Debug, Clone)]
pub struct AdminSubscriberRow {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub status: String,
    pub status_class: String,
    pub source: String,
    pub created_at: String,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct AdminMediaRow {
    pub id: i64,
    pub filename: String,
    pub url: String,
    pub mime: String,
    pub size: String,
    pub dimensions: String,
    pub alt_text: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AdminUserRow {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub role: String,
    pub role_class: String,
    pub is_active: bool,
    pub totp_enabled: bool,
    pub last_login_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AdminLogRow {
    pub id: i64,
    pub username: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub details: String,
    pub ip: String,
    pub created_at: String,
    pub created_iso: String,
}

#[derive(Debug, Clone)]
pub struct AdminSettingField {
    pub key: String,
    pub value: String,
    pub value_type: String,
    pub group_name: String,
    pub group_label: String,
    pub label: String,
    pub help_text: String,
}

#[derive(Debug, Clone)]
pub struct AdminSettingGroup {
    pub name: String,
    pub label: String,
    pub fields: Vec<AdminSettingField>,
}

#[derive(Debug, Clone)]
pub struct AdminRedirectRow {
    pub id: i64,
    pub from_path: String,
    pub to_path: String,
    pub status_code: i64,
    pub hits: i64,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AdminCampaignRow {
    pub id: i64,
    pub subject: String,
    pub status: String,
    pub status_class: String,
    pub recipient_count: i64,
    pub sent_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AdminTagRow {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub color: String,
    pub post_count: i64,
}

#[derive(Debug, Clone)]
pub struct AdminCategoryRow {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub post_count: i64,
}

#[derive(Debug, Clone)]
pub struct ChartPoint {
    pub label: String,
    pub value: i64,
}

#[derive(Debug, Clone)]
pub struct AdminSessionRow {
    pub id: String,
    pub username: String,
    pub ip: String,
    pub user_agent: String,
    pub created_at: String,
    pub expires_at: String,
    pub is_current: bool,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl PostCard {
    pub fn from_summary(summary: &crate::models::db::PostSummary) -> Self {
        Self {
            id: summary.id,
            slug: summary.slug.clone(),
            title: summary.title.clone(),
            excerpt: summary.excerpt.clone(),
            url: format!("/blog/{}", summary.slug),
            cover_image: summary.cover_image.clone().unwrap_or_default(),
            has_cover: summary.cover_image.is_some(),
            category: summary.category_name.clone().unwrap_or_default(),
            category_slug: summary.category_slug.clone().unwrap_or_default(),
            category_name: summary.category_name.clone().unwrap_or_default(),
            tags: Vec::new(),
            author: summary.author_name.clone().unwrap_or_default(),
            author_initials: util::initials(summary.author_name.as_deref().unwrap_or("")),
            published_at: util::format_date(summary.published_at),
            published_iso: util::format_datetime(summary.published_at),
            time_ago: util::time_ago(summary.published_at),
            reading_time: summary.reading_time,
            view_count: summary.view_count,
            is_featured: summary.is_featured,
        }
    }
}

impl ProjectCard {
    pub fn from_model(project: &crate::models::db::Project) -> Self {
        let tech = project
            .tech_stack
            .as_deref()
            .and_then(|t| serde_json::from_str::<Vec<String>>(t).ok())
            .unwrap_or_default();

        Self {
            slug: project.slug.clone(),
            title: project.title.clone(),
            summary: project.summary.clone(),
            url: format!("/projects/{}", project.slug),
            cover_image: project.cover_image.clone().unwrap_or_default(),
            has_cover: project.cover_image.is_some(),
            repo_url: project.repo_url.clone().unwrap_or_default(),
            live_url: project.live_url.clone().unwrap_or_default(),
            has_repo: project.repo_url.is_some(),
            has_live: project.live_url.is_some(),
            is_featured: project.is_featured,
            tech,
            index: String::new(),
        }
    }
}

/// A small helper struct used by the admin list views.
#[derive(Debug, Clone, Serialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub selected: bool,
}

// ---------------------------------------------------------------------------
// Page views
// ---------------------------------------------------------------------------

/// A single entry in the pager.
#[derive(Debug, Clone)]
pub struct PageLink {
    pub number: i64,
    pub label: String,
    pub href: String,
    pub is_current: bool,
    pub is_gap: bool,
}

#[derive(Debug, Clone)]
pub struct PaginationView {
    pub page: usize,
    pub total_pages: usize,
    pub total: i64,
    pub per_page: usize,
    pub links: Vec<PageLink>,
    pub prev_href: String,
    pub next_href: String,
    pub base_path: String,
    pub query: String,
}

impl PaginationView {
    pub fn from_page(page: &util::Page<()>, base_path: &str, query: &str) -> Self {
        let suffix = if query.is_empty() {
            String::new()
        } else {
            format!("&{query}")
        };

        let build = |number: usize| -> String { format!("{base_path}?page={number}{suffix}") };

        let links: Vec<PageLink> = page
            .window()
            .into_iter()
            .map(|number| {
                if number == 0 {
                    PageLink {
                        number: 0,
                        label: "…".to_string(),
                        href: String::new(),
                        is_current: false,
                        is_gap: true,
                    }
                } else {
                    PageLink {
                        number,
                        label: number.to_string(),
                        href: build(number as usize),
                        is_current: number as usize == page.page,
                        is_gap: false,
                    }
                }
            })
            .collect();

        Self {
            page: page.page,
            total_pages: page.total_pages(),
            total: page.total,
            per_page: page.per_page,
            links,
            prev_href: if page.has_prev() {
                build(page.prev())
            } else {
                String::new()
            },
            next_href: if page.has_next() {
                build(page.next())
            } else {
                String::new()
            },
            base_path: base_path.to_string(),
            query: query.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FactView {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct HomeView {
    pub base: BaseContext,
    pub hero_title: String,
    pub hero_highlight: String,
    pub hero_subtitle: String,
    pub hero_words: Vec<String>,
    pub availability_note: String,
    pub stats: Vec<StatView>,
    pub featured_projects: Vec<ProjectCard>,
    pub latest_posts: Vec<PostCard>,
    pub featured_posts: Vec<PostCard>,
    pub skills: Vec<SkillGroup>,
    pub services: Vec<ServiceView>,
    pub testimonials: Vec<TestimonialView>,
    pub timeline: Vec<ExperienceView>,
    pub blog_count: i64,
    pub project_count: i64,
}

#[derive(Debug, Clone)]
pub struct AboutView {
    pub base: BaseContext,
    pub content_html: String,
    pub stats: Vec<StatView>,
    pub skills: Vec<SkillGroup>,
    pub timeline: Vec<ExperienceView>,
    pub facts: Vec<FactView>,
    pub services: Vec<ServiceView>,
    pub testimonials: Vec<TestimonialView>,
    pub photo_url: String,
}

#[derive(Debug, Clone)]
pub struct ProjectsView {
    pub base: BaseContext,
    pub projects: Vec<ProjectCard>,
    pub tech: Vec<String>,
    pub active_tech: String,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct ProjectDetailView {
    pub base: BaseContext,
    pub project: ProjectView,
}

#[derive(Debug, Clone)]
pub struct BlogIndexView {
    pub base: BaseContext,
    pub posts: Vec<PostCard>,
    pub pagination: PaginationView,
    pub categories: Vec<CategoryView>,
    pub tags: Vec<TagView>,
    pub heading: String,
    pub subheading: String,
    pub active_category: String,
    pub active_tag: String,
    pub query: String,
    pub total: i64,
    pub featured: Vec<PostCard>,
}

#[derive(Debug, Clone)]
pub struct BlogPostView {
    pub base: BaseContext,
    pub post: PostView,
    pub comments_enabled: bool,
    pub comment_error: String,
    pub comment_name: String,
    pub comment_email: String,
    pub comment_website: String,
}

#[derive(Debug, Clone)]
pub struct ExperiencePageView {
    pub base: BaseContext,
    pub work: Vec<ExperienceView>,
    pub education: Vec<ExperienceView>,
    pub skills: Vec<SkillGroup>,
    pub stats: Vec<StatView>,
}

#[derive(Debug, Clone)]
pub struct SkillsView {
    pub base: BaseContext,
    pub skills: Vec<SkillGroup>,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct ServicesView {
    pub base: BaseContext,
    pub services: Vec<ServiceView>,
    pub process: Vec<FactView>,
    pub testimonials: Vec<TestimonialView>,
}

#[derive(Debug, Clone)]
pub struct TestimonialsView {
    pub base: BaseContext,
    pub testimonials: Vec<TestimonialView>,
    pub stats: Vec<StatView>,
}

#[derive(Debug, Clone)]
pub struct AwardsView {
    pub base: BaseContext,
    pub awards: Vec<AwardView>,
    pub talks: Vec<TalkView>,
    pub publications: Vec<PublicationView>,
}

#[derive(Debug, Clone)]
pub struct SpeakingView {
    pub base: BaseContext,
    pub talks: Vec<TalkView>,
    pub publications: Vec<PublicationView>,
}

#[derive(Debug, Clone)]
pub struct PublicationsView {
    pub base: BaseContext,
    pub publications: Vec<PublicationView>,
    pub talks: Vec<TalkView>,
}

#[derive(Debug, Clone)]
pub struct GalleryPageView {
    pub base: BaseContext,
    pub items: Vec<GalleryViewItem>,
    pub categories: Vec<String>,
    pub active_category: String,
}

#[derive(Debug, Clone)]
pub struct GalleryViewItem {
    pub title: String,
    pub image: String,
    pub caption: String,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct ContactView {
    pub base: BaseContext,
    pub success: bool,
    pub message: String,
    pub errors: Vec<String>,
    pub form_name: String,
    pub form_email: String,
    pub form_subject: String,
    pub form_message: String,
    pub socials: Vec<SocialView>,
}

#[derive(Debug, Clone)]
pub struct ResumeView {
    pub base: BaseContext,
    pub work: Vec<ExperienceView>,
    pub education: Vec<ExperienceView>,
    pub skills: Vec<SkillGroup>,
    pub stats: Vec<StatView>,
    pub cv_file: String,
}

#[derive(Debug, Clone)]
pub struct PageView {
    pub base: BaseContext,
    pub title: String,
    pub content_html: String,
    pub is_full_width: bool,
}

#[derive(Debug, Clone)]
pub struct SearchView {
    pub base: BaseContext,
    pub query: String,
    pub posts: Vec<PostCard>,
    pub projects: Vec<ProjectCard>,
    pub pages: Vec<PageViewItem>,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct PageViewItem {
    pub title: String,
    pub url: String,
    pub excerpt: String,
}

#[derive(Debug, Clone)]
pub struct NewsletterView {
    pub base: BaseContext,
    pub heading: String,
    pub subheading: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AuthView {
    pub base: BaseContext,
    pub error: String,
    pub username: String,
    pub next: String,
    pub totp_required: bool,
    pub show_2fa_hint: bool,
}

#[derive(Debug, Clone)]
pub struct LogoutView {
    pub base: BaseContext,
}

// ---------------------------------------------------------------------------
// Admin
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AdminContext {
    pub active: String,
    pub unread_messages: i64,
    pub pending_comments: i64,
    pub draft_posts: i64,
    pub subscriber_count: i64,
    pub version: String,
    pub uptime: String,
}

#[derive(Debug, Clone)]
pub struct DashboardView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub stats: AdminStats,
    pub recent_posts: Vec<AdminPostRow>,
    pub pending_comments: Vec<AdminCommentRow>,
    pub new_messages: Vec<AdminMessageRow>,
    pub traffic: Vec<ChartPoint>,
    pub top_posts: Vec<AdminPostRow>,
    pub top_paths: Vec<ChartPoint>,
    pub browser_stats: Vec<ChartPoint>,
}

#[derive(Debug, Clone)]
pub struct AdminPostsView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub posts: Vec<AdminPostRow>,
    pub pagination: PaginationView,
    pub status_filter: String,
    pub query: String,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct AdminPostFormView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub post: Option<AdminPostEdit>,
    pub categories: Vec<AdminCategoryRow>,
    pub tags: Vec<AdminTagRow>,
    pub all_tags: String,
    pub is_new: bool,
    pub csrf_token: String,
    pub error: String,
    pub preview_html: String,
}

#[derive(Debug, Clone)]
pub struct AdminPostEdit {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub content_md: String,
    pub content_html: String,
    pub status: String,
    pub category_id: Option<i64>,
    pub category_id_str: String,
    pub cover_image: String,
    pub is_featured: bool,
    pub allow_comments: bool,
    pub meta_title: String,
    pub meta_description: String,
    pub published_at: String,
    pub scheduled_for: String,
    pub reading_time: i64,
    pub word_count: usize,
    pub view_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct AdminPagesView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub pages: Vec<AdminPageRow>,
}

#[derive(Debug, Clone)]
pub struct AdminPageFormView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub page: Option<AdminPageEdit>,
    pub is_new: bool,
    pub csrf_token: String,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct AdminPageEdit {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub content_md: String,
    pub content_html: String,
    pub status: String,
    pub is_full_width: bool,
    pub show_in_nav: bool,
    pub nav_order: i64,
    pub meta_title: String,
    pub meta_description: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct AdminProjectsView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub projects: Vec<AdminProjectRow>,
}

#[derive(Debug, Clone)]
pub struct AdminProjectFormView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub project: Option<AdminProjectEdit>,
    pub is_new: bool,
    pub csrf_token: String,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct AdminProjectEdit {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub description_md: String,
    pub description_html: String,
    pub cover_image: String,
    pub repo_url: String,
    pub live_url: String,
    pub client: String,
    pub status: String,
    pub is_featured: bool,
    pub sort_order: i64,
    pub tech_stack: String,
    pub meta_title: String,
    pub meta_description: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct AdminTaxonomyView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub tags: Vec<AdminTagRow>,
    pub categories: Vec<AdminCategoryRow>,
}

#[derive(Debug, Clone)]
pub struct AdminCommentsView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub comments: Vec<AdminCommentRow>,
    pub status_filter: String,
    pub counts: Vec<ChartPoint>,
}

#[derive(Debug, Clone)]
pub struct AdminMessagesView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub messages: Vec<AdminMessageRow>,
    pub status_filter: String,
    pub counts: Vec<ChartPoint>,
}

#[derive(Debug, Clone)]
pub struct AdminSubscribersView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub subscribers: Vec<AdminSubscriberRow>,
    pub counts: Vec<ChartPoint>,
}

#[derive(Debug, Clone)]
pub struct AdminMediaView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub items: Vec<AdminMediaRow>,
    pub total_size: String,
}

#[derive(Debug, Clone)]
pub struct AdminUsersView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub users: Vec<AdminUserRow>,
    pub sessions: Vec<AdminSessionRow>,
}

#[derive(Debug, Clone)]
pub struct AdminUserFormView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub user: Option<AdminUserEdit>,
    pub is_new: bool,
    pub csrf_token: String,
    pub error: String,
    pub totp_secret: String,
    pub totp_uri: String,
    pub recovery_codes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AdminUserEdit {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub bio: String,
    pub avatar: String,
    pub is_active: bool,
    pub totp_enabled: bool,
    pub last_login_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AdminSettingsView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub groups: Vec<AdminSettingGroup>,
    pub csrf_token: String,
    pub saved: bool,
}

#[derive(Debug, Clone)]
pub struct AdminLogsView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub entries: Vec<AdminLogRow>,
    pub pagination: PaginationView,
}

#[derive(Debug, Clone)]
pub struct AdminRedirectsView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub redirects: Vec<AdminRedirectRow>,
    pub csrf_token: String,
}

#[derive(Debug, Clone)]
pub struct AdminNewsletterView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub campaigns: Vec<AdminCampaignRow>,
    pub subscriber_count: i64,
    pub csrf_token: String,
    pub sent: bool,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct AdminProfileView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub csrf_token: String,
    pub error: String,
    pub success: bool,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub bio: String,
    pub avatar: String,
    pub password_strength: u8,
    pub password_strength_label: String,
}

#[derive(Debug, Clone)]
pub struct AdminBackupView {
    pub ctx: AdminContext,
    pub base: BaseContext,
    pub stats: AdminStats,
    pub csrf_token: String,
}

#[derive(Debug, Clone)]
pub struct AdminTaxonomyEdit {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub color: String,
    pub post_count: i64,
}
