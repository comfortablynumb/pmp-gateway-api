mod clients;
mod config;
mod interpolation;
mod routes;

use anyhow::Result;
use clients::ClientManager;
use config::Config;
use routes::{build_router, handler::AppState};
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pmp_gateway_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting PMP Gateway API");

    // Load configuration
    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.yaml".to_string());
    info!("Loading configuration from: {}", config_path);

    let config = Config::from_yaml_file(&config_path)?;
    config.validate()?;

    info!(
        "Loaded configuration: {} clients, {} routes",
        config.clients.len(),
        config.routes.len()
    );

    // Initialize client manager
    let client_manager = ClientManager::from_config(&config)?;
    info!("Initialized client manager");

    // Create application state
    let state = AppState {
        config: Arc::new(config),
        client_manager: Arc::new(client_manager),
    };

    // Build router
    let app = build_router(state).layer(TraceLayer::new_for_http());

    // Determine bind address
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let bind_addr = format!("{}:{}", host, port);

    info!("Starting server on {}", bind_addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
