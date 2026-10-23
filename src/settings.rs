//! Site settings: typed access to the `settings` table with an in-memory cache.

use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Every configurable value of the site.
#[derive(Debug, Clone)]
pub struct Settings {
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
    pub default_og_image: String,
    pub cv_file: String,
    /// Primary site language: `fa` (Persian, RTL) or `en` (English).
    pub default_locale: String,
    // English variants of site-wide copy, used when the active locale is English.
    pub subtitle_en: String,
    pub location_en: String,
    pub description_en: String,
    pub tagline_en: String,
    pub availability_note_en: String,
    pub newsletter_heading_en: String,
    pub newsletter_subheading_en: String,
    pub contact_success_message_en: String,

    pub theme_accent: String,
    pub theme_accent_2: String,
    pub default_theme: String,
    pub enable_dark_mode: bool,

    pub posts_per_page: usize,
    pub enable_comments: bool,
    pub enable_newsletter: bool,
    pub allow_registration: bool,

    pub seo_meta_description: String,
    pub seo_og_type: String,
    pub seo_twitter_site: String,
    pub seo_google_site_verification: String,
    pub seo_bing_verification: String,
    pub seo_person_name: String,
    pub seo_person_name_en: String,

    pub analytics_provider: String,
    pub analytics_plausible_domain: String,
    pub analytics_ga_measurement_id: String,
    pub analytics_custom_head: String,
    pub analytics_custom_footer: String,

    pub contact_form_enabled: bool,
    pub contact_success_message: String,
    pub contact_notify_email: String,

    pub newsletter_heading: String,
    pub newsletter_subheading: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            title: "Personal Site".into(),
            subtitle: "Engineer".into(),
            description: String::new(),
            tagline: String::new(),
            email: String::new(),
            phone: String::new(),
            location: String::new(),
            available: true,
            availability_note: String::new(),
            logo_text: "PS".into(),
            favicon: "/assets/img/favicon.svg".into(),
            default_og_image: "/assets/img/og-default.svg".into(),
            cv_file: String::new(),
            default_locale: "fa".into(),
            subtitle_en: String::new(),
            location_en: String::new(),
            description_en: String::new(),
            tagline_en: String::new(),
            availability_note_en: String::new(),
            newsletter_heading_en: String::new(),
            newsletter_subheading_en: String::new(),
            contact_success_message_en: String::new(),
            theme_accent: "#6366f1".into(),
            theme_accent_2: "#22d3ee".into(),
            default_theme: "dark".into(),
            enable_dark_mode: true,
            posts_per_page: 9,
            enable_comments: true,
            enable_newsletter: true,
            allow_registration: false,
            seo_meta_description: String::new(),
            seo_og_type: "website".into(),
            seo_twitter_site: String::new(),
            seo_google_site_verification: String::new(),
            seo_bing_verification: String::new(),
            seo_person_name: String::new(),
            seo_person_name_en: String::new(),
            analytics_provider: "none".into(),
            analytics_plausible_domain: String::new(),
            analytics_ga_measurement_id: String::new(),
            analytics_custom_head: String::new(),
            analytics_custom_footer: String::new(),
            contact_form_enabled: true,
            contact_success_message: "Thanks for getting in touch!".into(),
            contact_notify_email: String::new(),
            newsletter_heading: "Subscribe".into(),
            newsletter_subheading: String::new(),
        }
    }
}

impl Settings {
    /// Build a `Settings` value from a raw key/value map.
    pub fn from_map(map: &HashMap<String, String>) -> Self {
        let mut s = Settings::default();

        macro_rules! get {
            ($key:expr) => {
                map.get($key).cloned().unwrap_or_default()
            };
        }
        macro_rules! set_str {
            ($field:ident, $key:expr) => {
                s.$field = get!($key);
            };
        }
        macro_rules! set_bool {
            ($field:ident, $key:expr) => {
                s.$field = matches!(get!($key).as_str(), "1" | "true" | "yes" | "on");
            };
        }
        macro_rules! set_num {
            ($field:ident, $key:expr) => {
                s.$field = get!($key).parse().unwrap_or(s.$field);
            };
        }

        set_str!(title, "site.title");
        set_str!(subtitle, "site.subtitle");
        set_str!(description, "site.description");
        set_str!(tagline, "site.tagline");
        set_str!(email, "site.email");
        set_str!(phone, "site.phone");
        set_str!(location, "site.location");
        set_bool!(available, "site.available");
        set_str!(availability_note, "site.availability_note");
        set_str!(logo_text, "site.logo_text");
        set_str!(favicon, "site.favicon");
        set_str!(default_og_image, "site.default_og_image");
        set_str!(cv_file, "site.cv_file");
        set_str!(default_locale, "site.default_locale");
        set_str!(subtitle_en, "site.subtitle_en");
        set_str!(location_en, "site.location_en");
        set_str!(description_en, "site.description_en");
        set_str!(tagline_en, "site.tagline_en");
        set_str!(availability_note_en, "site.availability_note_en");
        set_str!(newsletter_heading_en, "newsletter.heading_en");
        set_str!(newsletter_subheading_en, "newsletter.subheading_en");
        set_str!(contact_success_message_en, "contact.success_message_en");

        set_str!(theme_accent, "site.theme_accent");
        set_str!(theme_accent_2, "site.theme_accent_2");
        set_str!(default_theme, "site.default_theme");
        set_bool!(enable_dark_mode, "site.enable_dark_mode");

        set_num!(posts_per_page, "site.posts_per_page");
        set_bool!(enable_comments, "site.enable_comments");
        set_bool!(enable_newsletter, "site.enable_newsletter");
        set_bool!(allow_registration, "site.allow_registration");

        set_str!(seo_meta_description, "seo.meta_description");
        set_str!(seo_og_type, "seo.og_type");
        set_str!(seo_twitter_site, "seo.twitter_site");
        set_str!(seo_google_site_verification, "seo.google_site_verification");
        set_str!(seo_bing_verification, "seo.bing_verification");
        set_str!(seo_person_name, "seo.person_name");
        set_str!(seo_person_name_en, "seo.person_name_en");

        set_str!(analytics_provider, "analytics.provider");
        set_str!(analytics_plausible_domain, "analytics.plausible_domain");
        set_str!(analytics_ga_measurement_id, "analytics.ga_measurement_id");
        set_str!(analytics_custom_head, "analytics.custom_head");
        set_str!(analytics_custom_footer, "analytics.custom_footer");

        set_bool!(contact_form_enabled, "contact.form_enabled");
        set_str!(contact_success_message, "contact.success_message");
        set_str!(contact_notify_email, "contact.notify_email");

        set_str!(newsletter_heading, "newsletter.heading");
        set_str!(newsletter_subheading, "newsletter.subheading");

        s
    }
}

/// Load all settings from the database.
pub async fn load(pool: &SqlitePool) -> sqlx::Result<Settings> {
    let rows: Vec<(String, Option<String>)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(pool)
        .await?;

    let mut map = HashMap::new();
    for (key, value) in rows {
        if let Some(value) = value {
            map.insert(key, value);
        }
    }
    Ok(Settings::from_map(&map))
}

/// A thread-safe, cheaply cloneable handle to the current settings.
#[derive(Clone)]
pub struct SettingsHandle(Arc<RwLock<Settings>>);

impl SettingsHandle {
    pub fn new(settings: Settings) -> Self {
        Self(Arc::new(RwLock::new(settings)))
    }

    /// Read a snapshot of the settings.
    pub fn snapshot(&self) -> Settings {
        self.0.read().expect("settings lock poisoned").clone()
    }

    /// Replace the current settings.
    pub fn replace(&self, settings: Settings) {
        *self.0.write().expect("settings lock poisoned") = settings;
    }

    /// Refresh the cache from the database.
    pub async fn refresh(&self, pool: &SqlitePool) -> sqlx::Result<()> {
        let settings = load(pool).await?;
        self.replace(settings);
        Ok(())
    }
}

/// Persist a single setting.
pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(key)
    .bind(value)
    .bind(Utc::now())
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch a raw setting value.
pub async fn get(pool: &SqlitePool, key: &str) -> sqlx::Result<Option<String>> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(value)
}
