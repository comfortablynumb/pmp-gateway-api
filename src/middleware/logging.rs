use axum::{extract::Request, middleware::Next, response::Response};
use std::sync::Arc;
use tracing::{debug, info};

use crate::config::LoggingConfig;

/// Middleware to log requests and responses
pub async fn logging_middleware(
    config: Arc<LoggingConfig>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let request_id = request
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    // Log request
    info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        "Incoming request"
    );

    if config.log_headers {
        debug!(request_id = %request_id, headers = ?request.headers(), "Request headers");
    }

    // Call next middleware/handler
    let response = next.run(request).await;

    // Log response
    info!(
        request_id = %request_id,
        status = response.status().as_u16(),
        "Response sent"
    );

    response
}

/// Create logging middleware with config
pub fn create_logging_middleware(
    config: LoggingConfig,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone {
    let config = Arc::new(config);
    move |request: Request, next: Next| {
        let config = config.clone();
        Box::pin(async move { logging_middleware(config, request, next).await })
    }
}
