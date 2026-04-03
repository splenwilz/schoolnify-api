use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file (silently ignore if missing)
    dotenvy::dotenv().ok();

    // Initialize structured logging
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "schoolnify_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting schoolnify-api");

    // Load configuration
    let config = schoolnify_api::config::AppConfig::load()?;
    tracing::info!(
        host = %config.server.host,
        port = %config.server.port,
        "Configuration loaded"
    );

    // Connect to database
    let db_pool = schoolnify_api::db::create_pool(&config.database).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&db_pool).await?;
    tracing::info!("Database migrations applied");

    // Build app
    let state = schoolnify_api::state::AppState::new(config.clone(), db_pool);
    let app = schoolnify_api::build_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind((config.server.host.as_str(), config.server.port)).await?;
    let addr = listener.local_addr()?;
    tracing::info!(%addr, "Server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received Ctrl+C"),
        _ = terminate => tracing::info!("Received SIGTERM"),
    }
}
