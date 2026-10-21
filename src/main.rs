//! Entry point: configuration, database, router and graceful shutdown.

use std::net::SocketAddr;
use std::sync::Arc;

use personal_site::config::Config;
use personal_site::db;
use personal_site::handlers;
use personal_site::mail::Mailer;
use personal_site::middleware::security;
use personal_site::settings::{self, SettingsHandle};
use personal_site::state::AppState;

#[tokio::main]
async fn main() {
    if dotenvy::dotenv().is_err() {
        tracing::debug!("no .env file found — using the process environment");
    }

    init_tracing();

    if let Err(err) = run().await {
        tracing::error!("fatal: {err}");
        std::process::exit(1);
    }
}

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,tower_http=warn,sqlx=warn"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_level(true)
        .compact()
        .init();
}

async fn run() -> personal_site::config::anyhow_lite::Result<()> {
    let config = Config::from_env()?;
    config.validate()?;

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        environment = ?config.environment,
        "starting personal-site"
    );

    // Create the media directory.
    std::fs::create_dir_all(&config.media_dir)?;

    let pool = db::connect(&config).await?;
    db::seed::run(&pool, &config).await?;

    let initial_settings = settings::load(&pool).await?;
    let settings_handle = SettingsHandle::new(initial_settings);

    let mailer = Mailer::new(config.smtp.clone());

    let state = AppState::new(
        pool.clone(),
        Arc::new(config.clone()),
        settings_handle,
        mailer,
    );

    // Publish scheduled posts whose time has come.
    publish_due_posts(&pool).await;

    // Background maintenance.
    let maintenance_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            ticker.tick().await;
            let purged =
                personal_site::auth::session::purge_expired_sessions(&maintenance_state.db).await;
            if purged > 0 {
                tracing::info!(count = purged, "purged expired sessions");
            }
            publish_due_posts(&maintenance_state.db).await;
        }
    });

    let app = handlers::router(state);

    let addr: SocketAddr = config.bind;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);

    let serve = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal());

    if let Err(err) = serve.await {
        tracing::error!("server error: {err}");
    }

    tracing::info!("shutdown complete");
    Ok(())
}

/// Move scheduled posts to `published` once their time has arrived.
async fn publish_due_posts(pool: &sqlx::SqlitePool) {
    let result = sqlx::query(
        "UPDATE posts SET status = 'published', published_at = scheduled_for, updated_at = ?
         WHERE status = 'scheduled' AND scheduled_for IS NOT NULL AND scheduled_for <= ?",
    )
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now())
    .execute(pool)
    .await;

    if let Ok(result) = result {
        if result.rows_affected() > 0 {
            tracing::info!(count = result.rows_affected(), "published scheduled posts");
        }
    }
}

/// Wait for Ctrl-C or SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received — finishing in-flight requests");
}

/// Re-export so the binary can use the security header helper.
#[allow(dead_code)]
fn _security_headers() {
    let _ = security::apply;
}
