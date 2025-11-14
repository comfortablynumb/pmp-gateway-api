use crate::clients::ClientManager;
use crate::config::{Config, SubrequestTypeConfig};
use crate::interpolation::InterpolationContext;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Shared application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub client_manager: Arc<ClientManager>,
}

/// Generic route handler that processes subrequests
pub async fn handle_route(
    State(state): State<AppState>,
    method: Method,
    Path(path_params): Path<HashMap<String, String>>,
    Query(query_params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: String,
) -> Result<Response, AppError> {
    info!(
        "Handling request: {} with {} path params, {} query params",
        method,
        path_params.len(),
        query_params.len()
    );

    // Find the matching route configuration
    // For now, we'll execute all subrequests for simplicity
    // In a real implementation, you'd match the route based on path and method

    // Create interpolation context
    let context = InterpolationContext::new(
        headers.clone(),
        path_params,
        query_params,
        Some(body),
        method.clone(),
    );

    // Collect results from all subrequests
    let mut results = Vec::new();

    // For demonstration, let's find the first route that matches the method
    // A more sophisticated implementation would do proper path matching
    if let Some(route_config) = state.config.routes.first() {
        for subrequest in &route_config.subrequests {
            debug!("Executing subrequest for client: {}", subrequest.client_id);

            match &subrequest.config {
                SubrequestTypeConfig::Http(http_config) => {
                    // Get the HTTP client
                    let client = state
                        .client_manager
                        .get_http_client(&subrequest.client_id)
                        .ok_or_else(|| {
                            AppError::ClientNotFound(subrequest.client_id.clone())
                        })?;

                    // Interpolate URI
                    let uri = context.interpolate(&http_config.uri);

                    // Interpolate headers
                    let headers: HashMap<String, String> = http_config
                        .headers
                        .iter()
                        .map(|(k, v)| (k.clone(), context.interpolate(v)))
                        .collect();

                    // Interpolate body
                    let body = http_config
                        .body
                        .as_ref()
                        .map(|b| context.interpolate(b));

                    // Interpolate query params
                    let query_params: HashMap<String, String> = http_config
                        .query_params
                        .iter()
                        .map(|(k, v)| (k.clone(), context.interpolate(v)))
                        .collect();

                    // Execute the HTTP request
                    let response = client
                        .execute_request(
                            &http_config.method,
                            &uri,
                            headers,
                            body,
                            query_params,
                        )
                        .await
                        .map_err(|e| AppError::SubrequestFailed(e.to_string()))?;

                    results.push(json!({
                        "client_id": subrequest.client_id,
                        "status": response.status,
                        "body": response.body,
                        "headers": response.headers,
                    }));
                }
            }
        }
    }

    // Return aggregated results
    let response_body = json!({
        "subrequests": results,
        "count": results.len(),
    });

    Ok((StatusCode::OK, axum::Json(response_body)).into_response())
}

/// Application error types
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Client not found: {0}")]
    ClientNotFound(String),

    #[error("Subrequest failed: {0}")]
    SubrequestFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::ClientNotFound(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::SubrequestFailed(ref msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::InvalidConfig(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        error!("Request failed: {}", error_message);

        let body = json!({
            "error": error_message,
        });

        (status, axum::Json(body)).into_response()
    }
}
