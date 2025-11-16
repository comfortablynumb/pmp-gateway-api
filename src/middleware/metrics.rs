use axum::{extract::Request, middleware::Next, response::Response};
use metrics::{counter, describe_counter, describe_histogram, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::sync::OnceLock;
use std::time::Instant;

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Initialize Prometheus metrics exporter
pub fn init_metrics() -> PrometheusHandle {
    let builder = PrometheusBuilder::new();

    // Configure histogram buckets for latency metrics
    let builder = builder
        .set_buckets_for_metric(
            Matcher::Full("http_request_duration_seconds".to_string()),
            &[
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        )
        .expect("failed to set buckets");

    let handle = builder
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    // Describe metrics
    describe_counter!("http_requests_total", "Total number of HTTP requests");
    describe_counter!(
        "http_responses_total",
        "Total number of HTTP responses by status"
    );
    describe_counter!("http_errors_total", "Total number of HTTP errors");
    describe_histogram!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds"
    );

    PROMETHEUS_HANDLE.set(handle.clone()).ok();
    handle
}

/// Get the Prometheus handle
pub fn get_metrics_handle() -> Option<&'static PrometheusHandle> {
    PROMETHEUS_HANDLE.get()
}

/// Middleware to collect metrics for requests
pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    // Increment request counter
    counter!("http_requests_total", "method" => method.clone(), "path" => path.clone())
        .increment(1);

    // Call next middleware/handler
    let response = next.run(request).await;

    // Record request duration
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16();
    let status_class = format!("{}xx", status / 100);

    histogram!(
        "http_request_duration_seconds",
        "method" => method.clone(),
        "path" => path.clone(),
        "status" => status.to_string()
    )
    .record(duration);

    // Increment response counter by status
    counter!(
        "http_responses_total",
        "method" => method,
        "path" => path,
        "status" => status.to_string(),
        "status_class" => status_class.clone()
    )
    .increment(1);

    // Increment error counter for 4xx and 5xx
    if status >= 400 {
        counter!(
            "http_errors_total",
            "status_class" => status_class
        )
        .increment(1);
    }

    response
}

/// Handler for Prometheus metrics endpoint
pub async fn metrics_handler() -> String {
    if let Some(handle) = get_metrics_handle() {
        handle.render()
    } else {
        "# Metrics not initialized\n".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        let handle = init_metrics();

        // Record some test metrics
        counter!("http_requests_total", "method" => "GET", "path" => "/test").increment(1);
        histogram!("http_request_duration_seconds", "method" => "GET", "path" => "/test", "status" => "200").record(0.5);

        let output = handle.render();

        // Check that metrics are being tracked
        assert!(
            !output.is_empty(),
            "Metrics output should not be empty after recording"
        );
        assert!(
            output.contains("http_requests_total"),
            "Should contain request counter"
        );
        assert!(
            output.contains("http_request_duration_seconds"),
            "Should contain duration histogram"
        );
    }
}
