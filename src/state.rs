//! Application state shared by every handler.

use sqlx::SqlitePool;
use std::sync::Arc;

use crate::config::Config;
use crate::mail::Mailer;
use crate::middleware::ratelimit::RateLimiter;
use crate::settings::{Settings, SettingsHandle};

/// Everything a handler needs to do its job.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
    pub settings: SettingsHandle,
    pub mailer: Mailer,
    pub limiter: Arc<RateLimiter>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub version: &'static str,
}

impl AppState {
    pub fn new(
        db: SqlitePool,
        config: Arc<Config>,
        settings: SettingsHandle,
        mailer: Mailer,
    ) -> Self {
        Self {
            db,
            config,
            settings,
            mailer,
            limiter: Arc::new(RateLimiter::new()),
            started_at: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    /// A snapshot of the current site settings.
    pub fn settings_snapshot(&self) -> Settings {
        self.settings.snapshot()
    }

    /// Whether cookies should carry the `Secure` flag.
    pub fn use_secure_cookies(&self) -> bool {
        self.config.environment.is_production() || self.config.base_url.starts_with("https://")
    }
}
