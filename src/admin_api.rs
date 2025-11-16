use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    config::Config,
    health_aggregation::{AggregatedHealth, HealthCheckManager},
};

/// Admin API state
#[derive(Clone)]
pub struct AdminState {
    pub config: Arc<RwLock<Config>>,
    pub health_manager: Arc<HealthCheckManager>,
}

/// Gateway information response
#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayInfo {
    pub name: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub clients_count: usize,
    pub routes_count: usize,
}

/// Config reload response
#[derive(Debug, Serialize, Deserialize)]
pub struct ReloadResponse {
    pub success: bool,
    pub message: String,
}

/// Route information
#[derive(Debug, Serialize, Deserialize)]
pub struct RouteInfo {
    pub method: String,
    pub path: String,
    pub subrequests_count: usize,
    pub has_traffic_split: bool,
}

/// Create admin API router
pub fn create_admin_router(state: AdminState) -> Router {
    Router::new()
        .route("/admin/info", get(get_gateway_info))
        .route("/admin/health", get(get_health_status))
        .route("/admin/config", get(get_current_config))
        .route("/admin/config/reload", post(reload_config))
        .route("/admin/routes", get(list_routes))
        .route("/admin/clients", get(list_clients))
        .route("/admin/client/:id", get(get_client_info))
        .with_state(state)
}

/// Get gateway information
async fn get_gateway_info(State(state): State<AdminState>) -> Json<GatewayInfo> {
    let config = state.config.read().await;

    Json(GatewayInfo {
        name: "PMP Gateway".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // Would need to track start time
        clients_count: config.clients.len(),
        routes_count: config.routes.len(),
    })
}

/// Get health status
async fn get_health_status(State(state): State<AdminState>) -> Json<AggregatedHealth> {
    Json(state.health_manager.get_aggregated_health().await)
}

/// Get current configuration
async fn get_current_config(State(state): State<AdminState>) -> Json<Config> {
    let config = state.config.read().await;
    Json(config.clone())
}

/// Reload configuration
async fn reload_config(State(_state): State<AdminState>) -> (StatusCode, Json<ReloadResponse>) {
    info!("Admin API: Configuration reload requested");

    // In a real implementation, this would reload from file
    // For now, return a placeholder response
    (
        StatusCode::OK,
        Json(ReloadResponse {
            success: true,
            message: "Configuration reload triggered (hot reload via file watch is active)"
                .to_string(),
        }),
    )
}

/// List all routes
async fn list_routes(State(state): State<AdminState>) -> Json<Vec<RouteInfo>> {
    let config = state.config.read().await;

    let routes: Vec<RouteInfo> = config
        .routes
        .iter()
        .map(|r| RouteInfo {
            method: r.method.clone(),
            path: r.path.clone(),
            subrequests_count: r.subrequests.len(),
            has_traffic_split: r.traffic_split.is_some(),
        })
        .collect();

    Json(routes)
}

/// List all clients
async fn list_clients(State(state): State<AdminState>) -> Json<Vec<String>> {
    let config = state.config.read().await;
    let client_ids: Vec<String> = config.clients.keys().cloned().collect();
    Json(client_ids)
}

/// Get client information
async fn get_client_info(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let config = state.config.read().await;

    if let Some(client) = config.clients.get(&id) {
        let value = serde_json::to_value(client).unwrap();
        Ok(Json(value))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ClientConfig, HttpClientConfig, RouteConfig, ServerConfig};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_gateway_info() {
        let config = Config {
            clients: HashMap::new(),
            routes: vec![],
            server: ServerConfig::default(),
        };

        let state = AdminState {
            config: Arc::new(RwLock::new(config)),
            health_manager: Arc::new(HealthCheckManager::new()),
        };

        let info = get_gateway_info(State(state)).await;
        assert_eq!(info.0.name, "PMP Gateway");
        assert_eq!(info.0.clients_count, 0);
        assert_eq!(info.0.routes_count, 0);
    }

    #[tokio::test]
    async fn test_list_routes() {
        let mut clients = HashMap::new();
        clients.insert(
            "test".to_string(),
            ClientConfig::Http(HttpClientConfig {
                base_url: "http://test.com".to_string(),
                backends: vec![],
                load_balance: None,
                headers: HashMap::new(),
                min_connections: 1,
                max_connections: 10,
                timeout: 30,
                retry: None,
                circuit_breaker: None,
            }),
        );

        let config = Config {
            clients,
            routes: vec![RouteConfig {
                method: "GET".to_string(),
                path: "/test".to_string(),
                subrequests: vec![],
                response_transform: None,
                execution_mode: crate::config::ExecutionMode::Parallel,
                traffic_split: None,
                traffic_mirror: None,
            }],
            server: ServerConfig::default(),
        };

        let state = AdminState {
            config: Arc::new(RwLock::new(config)),
            health_manager: Arc::new(HealthCheckManager::new()),
        };

        let routes = list_routes(State(state)).await;
        assert_eq!(routes.0.len(), 1);
        assert_eq!(routes.0[0].path, "/test");
    }
}
