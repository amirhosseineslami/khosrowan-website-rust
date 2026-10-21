//! Personal website & CMS — a single Rust binary.
//!
//! The crate is organised in layers:
//!
//! * `config`, `settings` — environment and database-backed configuration.
//! * `db`, `models` — persistence.
//! * `auth` — passwords, sessions and TOTP.
//! * `handlers` — HTTP endpoints, split into public and admin areas.
//! * `middleware` — cross-cutting concerns (sessions, rate limiting, headers).
//! * `templates`, `views` — Askama templates and their view models.

pub mod audit;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod extract;
pub mod handlers;
pub mod mail;
pub mod markdown;
pub mod middleware;
pub mod models;
pub mod settings;
pub mod state;
pub mod templates;
pub mod util;
pub mod views;

pub use audit::audit;
pub use config::Config;
pub use error::{AppError, AppResult};
pub use state::AppState;
