use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::Response,
};
use opentelemetry::{
    global,
    trace::{Span, SpanKind, Status, Tracer},
    KeyValue,
};
use opentelemetry_sdk::{
    trace::{Config, TracerProvider},
    Resource,
};
use std::time::SystemTime;
use tracing::info;

/// OpenTelemetry configuration
#[derive(Debug, Clone)]
pub struct OtelConfig {
    /// Service name for tracing
    pub service_name: String,
    /// OTLP endpoint (e.g., "http://localhost:4317")
    pub otlp_endpoint: Option<String>,
    /// Enable tracing
    pub enabled: bool,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            service_name: "pmp-gateway".to_string(),
            otlp_endpoint: None,
            enabled: false,
        }
    }
}

/// Initialize OpenTelemetry tracing
pub fn init_tracing(config: &OtelConfig) -> Result<(), Box<dyn std::error::Error>> {
    if !config.enabled {
        info!("OpenTelemetry tracing is disabled");
        return Ok(());
    }

    info!("Initializing OpenTelemetry tracing with service: {}", config.service_name);

    // Create resource with service name
    let resource = Resource::new(vec![KeyValue::new(
        "service.name",
        config.service_name.clone(),
    )]);

    // Create tracer provider
    let tracer_provider = TracerProvider::builder()
        .with_config(Config::default().with_resource(resource))
        .build();

    // Set global tracer provider
    global::set_tracer_provider(tracer_provider.clone());

    // If OTLP endpoint is configured, we would set up the exporter here
    // For now, we're using the default provider which logs spans
    if let Some(ref endpoint) = config.otlp_endpoint {
        info!("OTLP endpoint configured: {}", endpoint);
        // In a full implementation, you would create an OTLP exporter here
        // and install it with the tracer provider
    }

    info!("OpenTelemetry tracing initialized successfully");
    Ok(())
}

/// Tracing middleware that creates spans for each request
pub async fn tracing_middleware(request: Request, next: Next) -> Response {
    let tracer = global::tracer("pmp-gateway");

    // Extract request information
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let version = format!("{:?}", request.version());

    // Create span
    let mut span = tracer
        .span_builder(format!("{} {}", method, path))
        .with_kind(SpanKind::Server)
        .with_start_time(SystemTime::now())
        .start(&tracer);

    // Add request attributes
    span.set_attribute(KeyValue::new("http.method", method.clone()));
    span.set_attribute(KeyValue::new("http.target", path.clone()));
    span.set_attribute(KeyValue::new("http.version", version));

    if let Some(host) = request.headers().get("host") {
        if let Ok(host_str) = host.to_str() {
            span.set_attribute(KeyValue::new("http.host", host_str.to_string()));
        }
    }

    if let Some(user_agent) = request.headers().get("user-agent") {
        if let Ok(ua_str) = user_agent.to_str() {
            span.set_attribute(KeyValue::new("http.user_agent", ua_str.to_string()));
        }
    }

    // Extract request ID if present
    if let Some(request_id) = request.headers().get("x-request-id") {
        if let Ok(id_str) = request_id.to_str() {
            span.set_attribute(KeyValue::new("request.id", id_str.to_string()));
        }
    }

    // Execute request
    let response = next.run(request).await;

    // Add response attributes
    let status_code = response.status().as_u16();
    span.set_attribute(KeyValue::new("http.status_code", status_code as i64));

    // Set span status based on HTTP status
    if status_code >= 500 {
        span.set_status(Status::error("Server error"));
    } else if status_code >= 400 {
        span.set_status(Status::error("Client error"));
    } else {
        span.set_status(Status::Ok);
    }

    // End span
    span.end();

    response
}

/// Shutdown OpenTelemetry gracefully
pub fn shutdown_tracing() {
    global::shutdown_tracer_provider();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otel_config_default() {
        let config = OtelConfig::default();
        assert_eq!(config.service_name, "pmp-gateway");
        assert!(!config.enabled);
        assert!(config.otlp_endpoint.is_none());
    }

    #[test]
    fn test_otel_config_custom() {
        let config = OtelConfig {
            service_name: "my-service".to_string(),
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            enabled: true,
        };
        assert_eq!(config.service_name, "my-service");
        assert!(config.enabled);
        assert!(config.otlp_endpoint.is_some());
    }

    #[test]
    fn test_init_tracing_disabled() {
        let config = OtelConfig {
            enabled: false,
            ..Default::default()
        };
        let result = init_tracing(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_tracing_enabled() {
        let config = OtelConfig {
            service_name: "test-service".to_string(),
            enabled: true,
            otlp_endpoint: None,
        };
        let result = init_tracing(&config);
        assert!(result.is_ok());
        shutdown_tracing();
    }
}
