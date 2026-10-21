//! Application configuration, loaded from the environment with sane defaults.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

/// Everything the application needs to know about its environment.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address the HTTP server binds to.
    pub bind: SocketAddr,
    /// Public base URL, used for canonical links, sitemap and feeds.
    pub base_url: String,
    /// SQLite database file location.
    pub database_url: String,
    /// Directory that holds uploaded media.
    pub media_dir: PathBuf,
    /// Directory that holds static assets (CSS/JS/images).
    pub assets_dir: PathBuf,
    /// Session cookie signing secret.
    pub secret: String,
    /// How long an idle session stays valid.
    pub session_ttl: Duration,
    /// How long a "remember me" session stays valid.
    pub remember_ttl: Duration,
    /// Bootstrap administrator username.
    pub admin_username: String,
    /// Bootstrap administrator e-mail.
    pub admin_email: String,
    /// Bootstrap administrator password (only used on an empty database).
    pub admin_password: String,
    /// Optional SMTP configuration for transactional mail.
    pub smtp: SmtpConfig,
    /// `development` enables verbose errors, auto-reload-ish logging and relaxed rules.
    pub environment: Environment,
    /// Maximum accepted request body size in bytes.
    pub max_body_size: usize,
    /// Maximum number of uploads accepted in a single multipart request.
    pub max_upload_size: usize,
    /// Whether the contact form should be accepted at all.
    pub contact_enabled: bool,
    /// Whether the public site should be visible to anonymous visitors.
    pub site_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    pub fn is_production(self) -> bool {
        matches!(self, Environment::Production)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SmtpConfig {
    pub host: Option<String>,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from: String,
    pub from_name: String,
    /// When true, mail is only printed to the log instead of being delivered.
    pub dry_run: bool,
}

impl SmtpConfig {
    pub fn is_configured(&self) -> bool {
        self.host.is_some()
    }
}

impl Config {
    /// Build the configuration from environment variables.
    pub fn from_env() -> anyhow_lite::Result<Self> {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());

        let host = env("HOST").unwrap_or_else(|| "0.0.0.0".to_string());
        let port: u16 = env("SERVER_PORT")
            .or_else(|| env("PORT"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(3000);
        let ip: IpAddr = host
            .parse()
            .unwrap_or(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));

        let environment = match env("APP_ENV")
            .unwrap_or_else(|| "development".into())
            .as_str()
        {
            "production" | "prod" => Environment::Production,
            _ => Environment::Development,
        };

        let secret =
            env("APP_SECRET").unwrap_or_else(|| "dev-only-insecure-secret-change-me".to_string());

        Ok(Self {
            bind: SocketAddr::new(ip, port),
            base_url: env("BASE_URL").unwrap_or_else(|| format!("http://localhost:{port}")),
            database_url: env("DATABASE_URL")
                .unwrap_or_else(|| "sqlite://data/personal-site.db?mode=rwc".to_string()),
            media_dir: PathBuf::from(env("MEDIA_DIR").unwrap_or_else(|| "media".into())),
            assets_dir: PathBuf::from(env("ASSETS_DIR").unwrap_or_else(|| "assets".into())),
            secret,
            session_ttl: Duration::from_secs(
                env("SESSION_TTL_SECONDS")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60 * 60 * 4),
            ),
            remember_ttl: Duration::from_secs(
                env("REMEMBER_TTL_SECONDS")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60 * 60 * 24 * 30),
            ),
            admin_username: env("ADMIN_USERNAME").unwrap_or_else(|| "admin".into()),
            admin_email: env("ADMIN_EMAIL").unwrap_or_else(|| "admin@example.com".into()),
            admin_password: env("ADMIN_PASSWORD").unwrap_or_else(|| "admin12345".into()),
            smtp: SmtpConfig {
                host: env("SMTP_HOST"),
                port: env("SMTP_PORT").and_then(|v| v.parse().ok()).unwrap_or(587),
                username: env("SMTP_USERNAME"),
                password: env("SMTP_PASSWORD"),
                from: env("SMTP_FROM").unwrap_or_else(|| "no-reply@example.com".into()),
                from_name: env("SMTP_FROM_NAME").unwrap_or_else(|| "Personal Site".into()),
                dry_run: env("SMTP_DRY_RUN")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(true),
            },
            environment,
            max_body_size: env("MAX_BODY_SIZE")
                .and_then(|v| v.parse().ok())
                .unwrap_or(2 * 1024 * 1024),
            max_upload_size: env("MAX_UPLOAD_SIZE")
                .and_then(|v| v.parse().ok())
                .unwrap_or(10 * 1024 * 1024),
            contact_enabled: env("CONTACT_ENABLED")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
            site_visible: env("SITE_VISIBLE")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
        })
    }

    /// A cryptographically strong secret is required in production.
    pub fn validate(&self) -> anyhow_lite::Result<()> {
        if self.environment.is_production() {
            if self.secret.len() < 32 {
                return Err(anyhow_lite::msg(
                    "APP_SECRET must be at least 32 characters in production",
                ));
            }
            if self.secret.starts_with("dev-only") {
                return Err(anyhow_lite::msg(
                    "APP_SECRET must not use the development default in production",
                ));
            }
            if self.admin_password == "admin12345" {
                return Err(anyhow_lite::msg(
                    "ADMIN_PASSWORD must be changed in production",
                ));
            }
        }
        Ok(())
    }
}

/// A tiny error type so the crate stays free of heavyweight dependencies in
/// the configuration layer.
pub mod anyhow_lite {
    pub type Result<T> = std::result::Result<T, Error>;

    /// Construct an [`Error`] from a message.
    pub fn msg(message: impl Into<String>) -> Error {
        Error(message.into())
    }

    #[derive(Debug)]
    pub struct Error(pub String);

    impl Error {
        pub fn msg(msg: impl Into<String>) -> Self {
            Self(msg.into())
        }
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for Error {}

    impl From<std::io::Error> for Error {
        fn from(e: std::io::Error) -> Self {
            Self(e.to_string())
        }
    }

    impl From<sqlx::Error> for Error {
        fn from(e: sqlx::Error) -> Self {
            Self(e.to_string())
        }
    }
}
