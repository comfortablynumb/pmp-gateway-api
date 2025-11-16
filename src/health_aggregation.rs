use axum::{http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error};

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Degraded,
}

/// Health check result for a single backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendHealth {
    /// Backend identifier
    pub backend_id: String,
    /// Current status
    pub status: HealthStatus,
    /// Last check timestamp
    pub last_check: String,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Error message if unhealthy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Aggregated health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedHealth {
    /// Overall gateway status
    pub status: HealthStatus,
    /// Individual backend statuses
    pub backends: HashMap<String, BackendHealth>,
    /// Total backends
    pub total_backends: usize,
    /// Healthy backends count
    pub healthy_count: usize,
    /// Unhealthy backends count
    pub unhealthy_count: usize,
}

/// Health checker configuration
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Health check interval in seconds
    pub interval: Duration,
    /// Health check timeout in seconds
    pub timeout: Duration,
    /// Number of consecutive failures before marking unhealthy
    pub failure_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            failure_threshold: 3,
        }
    }
}

/// Health check manager
pub struct HealthCheckManager {
    backends: Arc<RwLock<HashMap<String, BackendHealth>>>,
}

impl HealthCheckManager {
    /// Create a new health check manager
    pub fn new() -> Self {
        Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a backend for health checking
    pub async fn register_backend(&self, backend_id: String) {
        let mut backends = self.backends.write().await;
        backends.insert(
            backend_id.clone(),
            BackendHealth {
                backend_id,
                status: HealthStatus::Healthy,
                last_check: chrono::Utc::now().to_rfc3339(),
                response_time_ms: 0,
                error: None,
            },
        );
    }

    /// Update backend health status
    pub async fn update_backend_health(
        &self,
        backend_id: &str,
        status: HealthStatus,
        response_time_ms: u64,
        error: Option<String>,
    ) {
        let mut backends = self.backends.write().await;
        if let Some(backend) = backends.get_mut(backend_id) {
            backend.status = status;
            backend.last_check = chrono::Utc::now().to_rfc3339();
            backend.response_time_ms = response_time_ms;
            backend.error = error;
        }
    }

    /// Get aggregated health status
    pub async fn get_aggregated_health(&self) -> AggregatedHealth {
        let backends = self.backends.read().await;
        let total_backends = backends.len();
        let healthy_count = backends
            .values()
            .filter(|b| b.status == HealthStatus::Healthy)
            .count();
        let unhealthy_count = backends
            .values()
            .filter(|b| b.status == HealthStatus::Unhealthy)
            .count();

        // Determine overall status
        let status = if unhealthy_count == 0 {
            HealthStatus::Healthy
        } else if healthy_count == 0 {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Degraded
        };

        AggregatedHealth {
            status,
            backends: backends.clone(),
            total_backends,
            healthy_count,
            unhealthy_count,
        }
    }

    /// Start background health checking
    pub fn start_health_checks(
        self: Arc<Self>,
        config: HealthCheckConfig,
        check_fn: Arc<dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Duration, String>> + Send>> + Send + Sync>,
    ) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.interval);
            loop {
                interval.tick().await;
                debug!("Running health checks");

                let backend_ids: Vec<String> = {
                    let backends = self.backends.read().await;
                    backends.keys().cloned().collect()
                };

                for backend_id in backend_ids {
                    let self_clone = Arc::clone(&self);
                    let check_fn_clone = Arc::clone(&check_fn);
                    let backend_id_clone = backend_id.clone();

                    tokio::spawn(async move {
                        match check_fn_clone(backend_id_clone.clone()).await {
                            Ok(response_time) => {
                                self_clone
                                    .update_backend_health(
                                        &backend_id_clone,
                                        HealthStatus::Healthy,
                                        response_time.as_millis() as u64,
                                        None,
                                    )
                                    .await;
                            }
                            Err(error) => {
                                error!("Health check failed for {}: {}", backend_id_clone, error);
                                self_clone
                                    .update_backend_health(
                                        &backend_id_clone,
                                        HealthStatus::Unhealthy,
                                        0,
                                        Some(error),
                                    )
                                    .await;
                            }
                        }
                    });
                }
            }
        });
    }
}

impl Default for HealthCheckManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check endpoint handler
pub async fn health_check_handler(
    manager: Arc<HealthCheckManager>,
) -> (StatusCode, Json<AggregatedHealth>) {
    let health = manager.get_aggregated_health().await;

    let status_code = match health.status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::OK, // Still responding
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(health))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_manager_creation() {
        let manager = HealthCheckManager::new();
        let health = manager.get_aggregated_health().await;
        assert_eq!(health.total_backends, 0);
        assert_eq!(health.status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_register_backend() {
        let manager = HealthCheckManager::new();
        manager.register_backend("backend1".to_string()).await;

        let health = manager.get_aggregated_health().await;
        assert_eq!(health.total_backends, 1);
        assert_eq!(health.healthy_count, 1);
    }

    #[tokio::test]
    async fn test_update_backend_health() {
        let manager = HealthCheckManager::new();
        manager.register_backend("backend1".to_string()).await;

        manager
            .update_backend_health("backend1", HealthStatus::Unhealthy, 0, Some("Connection failed".to_string()))
            .await;

        let health = manager.get_aggregated_health().await;
        assert_eq!(health.unhealthy_count, 1);
        assert_eq!(health.status, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_degraded_status() {
        let manager = HealthCheckManager::new();
        manager.register_backend("backend1".to_string()).await;
        manager.register_backend("backend2".to_string()).await;

        manager
            .update_backend_health("backend2", HealthStatus::Unhealthy, 0, Some("Error".to_string()))
            .await;

        let health = manager.get_aggregated_health().await;
        assert_eq!(health.status, HealthStatus::Degraded);
        assert_eq!(health.healthy_count, 1);
        assert_eq!(health.unhealthy_count, 1);
    }
}
