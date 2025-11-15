use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Health check endpoint - returns OK if server is running
pub async fn health_check() -> Response {
    (StatusCode::OK, Json(json!({"status": "ok"}))).into_response()
}

/// Readiness check endpoint - returns OK if server is ready to accept traffic
pub async fn readiness_check() -> Response {
    // TODO: Add checks for database connections, etc.
    (StatusCode::OK, Json(json!({"status": "ready"}))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        // Response should be OK
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_readiness_check() {
        let response = readiness_check().await;
        // Response should be OK
        assert_eq!(response.status(), StatusCode::OK);
    }
}
