mod clients;
mod conditions;
mod config;
mod env_interpolation;
mod health;
mod interpolation;
mod middleware;
mod routes;
mod transform;

use anyhow::Result;
use axum::http::Method;
use clients::ClientManager;
use config::Config;
use routes::{build_router, handler::AppState};
use std::sync::Arc;
use std::time::Duration;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
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

    // Initialize Prometheus metrics
    middleware::init_metrics();
    info!("Initialized Prometheus metrics exporter");

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

    // Initialize client manager (now async)
    let client_manager = ClientManager::from_config(&config).await?;
    info!("Initialized client manager");

    // Create application state
    let state = AppState {
        config: Arc::new(config.clone()),
        client_manager: Arc::new(client_manager),
    };

    // Build router
    let mut app = build_router(state);

    // Apply CORS if configured
    if let Some(ref cors_config) = config.server.cors {
        info!("Enabling CORS");
        let mut cors = CorsLayer::new();

        // Set allowed origins
        if cors_config.allowed_origins.contains(&"*".to_string()) {
            cors = cors.allow_origin(Any);
        } else {
            for origin in &cors_config.allowed_origins {
                if let Ok(origin_header) = origin.parse::<axum::http::HeaderValue>() {
                    cors = cors.allow_origin(origin_header);
                }
            }
        }

        // Set allowed methods
        let methods: Vec<Method> = cors_config
            .allowed_methods
            .iter()
            .filter_map(|m| m.parse().ok())
            .collect();
        cors = cors.allow_methods(methods);

        // Set allowed headers
        if !cors_config.allowed_headers.is_empty() {
            let headers: Vec<_> = cors_config
                .allowed_headers
                .iter()
                .filter_map(|h| h.parse().ok())
                .collect();
            cors = cors.allow_headers(headers);
        } else {
            cors = cors.allow_headers(Any);
        }

        // Set credentials
        if cors_config.allow_credentials {
            cors = cors.allow_credentials(true);
        }

        // Set max age
        cors = cors.max_age(Duration::from_secs(cors_config.max_age));

        app = app.layer(cors);
    }

    // Apply request body size limit
    info!(
        "Setting max request body size: {} bytes",
        config.server.max_body_size
    );
    app = app.layer(RequestBodyLimitLayer::new(config.server.max_body_size));

    // Apply timeout
    info!("Setting request timeout: {} seconds", config.server.timeout);
    app = app.layer(TimeoutLayer::new(Duration::from_secs(
        config.server.timeout,
    )));

    // Apply rate limiting if configured
    if let Some(ref rate_limit_config) = config.server.rate_limit {
        info!(
            "Enabling rate limiting: {} req/s, burst: {}",
            rate_limit_config.requests_per_second, rate_limit_config.burst_size
        );
        let limiter = middleware::create_rate_limiter(rate_limit_config);
        app = app.layer(axum::middleware::from_fn(
            middleware::create_rate_limit_middleware(limiter),
        ));
    }

    // TODO: Apply security middleware if configured
    // Note: Security middleware requires ConnectInfo layer setup
    if config.server.security.api_keys.is_some()
        || config.server.security.jwt.is_some()
        || config.server.security.ip_filter.is_some()
    {
        info!("Security configuration detected (middleware integration pending)");
        // app = app.layer(...);
    }

    // Apply logging middleware
    app = app.layer(axum::middleware::from_fn(
        middleware::create_logging_middleware(config.server.logging.clone()),
    ));

    // Apply compression (gzip and brotli)
    info!("Enabling response compression (gzip, brotli)");
    app = app.layer(CompressionLayer::new());

    // Apply core middlewares
    app = app
        .layer(axum::middleware::from_fn(middleware::request_id_middleware))
        .layer(axum::middleware::from_fn(middleware::metrics_middleware))
        .layer(TraceLayer::new_for_http());

    // Determine bind address
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let bind_addr = format!("{}:{}", host, port);

    info!("Starting server on {}", bind_addr);

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server stopped gracefully");
    Ok(())
}

/// Handle shutdown signals for graceful termination
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C signal, shutting down gracefully");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM signal, shutting down gracefully");
        },
    }

    // Give connections time to finish
    tracing::info!("Draining connections...");
    tokio::time::sleep(Duration::from_secs(1)).await;
}
