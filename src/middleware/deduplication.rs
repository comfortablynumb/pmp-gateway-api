#![allow(dead_code)]
#![allow(private_interfaces)]

use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

/// Deduplication configuration
#[derive(Debug, Clone)]
pub struct DeduplicationConfig {
    /// Time window for deduplication
    pub ttl: Duration,
    /// Maximum number of tracked requests
    pub max_entries: u64,
}

impl Default for DeduplicationConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(60),
            max_entries: 10000,
        }
    }
}

/// Request deduplication key
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub(crate) struct DeduplicationKey {
    pub method: String,
    pub path: String,
    pub idempotency_key: Option<String>,
}

/// Cached response for deduplicated requests
#[derive(Debug, Clone)]
pub(crate) struct CachedResult {
    pub(crate) status: u16,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: bytes::Bytes,
}

/// Request deduplication middleware
pub struct RequestDeduplicator {
    pub(crate) cache: Cache<DeduplicationKey, CachedResult>,
}

impl RequestDeduplicator {
    /// Create a new request deduplicator
    pub fn new(config: DeduplicationConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.max_entries)
            .time_to_live(config.ttl)
            .build();

        Self { cache }
    }

    /// Check if request should be deduplicated and get cached response
    pub async fn check(&self, request: &Request) -> Option<CachedResult> {
        let key = self.extract_key(request)?;
        self.cache.get(&key).await
    }

    /// Store response for deduplication
    pub async fn store(&self, request: &Request, result: CachedResult) {
        if let Some(key) = self.extract_key(request) {
            self.cache.insert(key, result).await;
        }
    }

    /// Extract deduplication key from request
    fn extract_key(&self, request: &Request) -> Option<DeduplicationKey> {
        // Only deduplicate idempotent methods or requests with Idempotency-Key header
        let method = request.method().to_string();
        let is_idempotent = matches!(method.as_str(), "GET" | "HEAD" | "PUT" | "DELETE");

        let idempotency_key = request
            .headers()
            .get("idempotency-key")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if !is_idempotent && idempotency_key.is_none() {
            return None;
        }

        Some(DeduplicationKey {
            method: method.clone(),
            path: request.uri().path().to_string(),
            idempotency_key,
        })
    }
}

/// Create deduplication middleware
pub fn create_deduplication_middleware(
    dedup: Arc<RequestDeduplicator>,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone {
    move |request: Request, next: Next| {
        let dedup = dedup.clone();
        Box::pin(async move { deduplication_middleware(dedup, request, next).await })
    }
}

/// Deduplication middleware handler
async fn deduplication_middleware(
    dedup: Arc<RequestDeduplicator>,
    request: Request,
    next: Next,
) -> Response {
    // Extract request info first
    let request_method = request.method().to_string();
    let request_path = request.uri().path().to_string();
    let idempotency_key = request
        .headers()
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Check if we should deduplicate
    let is_idempotent = matches!(request_method.as_str(), "GET" | "HEAD" | "PUT" | "DELETE");
    if !is_idempotent && idempotency_key.is_none() {
        return next.run(request).await;
    }

    let dedup_key = DeduplicationKey {
        method: request_method.clone(),
        path: request_path.clone(),
        idempotency_key: idempotency_key.clone(),
    };

    // Check if we have a cached response
    if let Some(cached) = dedup.cache.get(&dedup_key).await {
        debug!("Request deduplicated: {} {}", request_method, request_path);

        // Build response from cache
        let mut response = Response::builder()
            .status(cached.status)
            .header("X-Deduplicated", "true");

        for (name, value) in &cached.headers {
            if let Ok(header_value) = value.parse::<axum::http::HeaderValue>() {
                response = response.header(name, header_value);
            }
        }

        return response
            .body(Body::from(cached.body.clone()))
            .unwrap_or_else(|_| Response::new(Body::empty()));
    }

    // Execute request
    let response = next.run(request).await;

    // Only cache successful responses
    let status = response.status();
    if !status.is_success() {
        return response;
    }

    // Extract response parts
    let (parts, body) = response.into_parts();

    // Read body
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Response::builder()
                .status(500)
                .body(Body::from("Failed to process response"))
                .unwrap();
        }
    };

    // Store in cache
    let headers: Vec<(String, String)> = parts
        .headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let cached_result = CachedResult {
        status: parts.status.as_u16(),
        headers: headers.clone(),
        body: body_bytes.clone(),
    };

    dedup.cache.insert(dedup_key, cached_result).await;

    // Build response with body
    let mut response = Response::builder().status(parts.status);

    for (name, value) in headers {
        if let Ok(header_value) = value.parse::<axum::http::HeaderValue>() {
            response = response.header(name, header_value);
        }
    }

    response
        .body(Body::from(body_bytes))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;

    #[tokio::test]
    async fn test_deduplicator_creation() {
        let config = DeduplicationConfig::default();
        let dedup = RequestDeduplicator::new(config);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        assert!(dedup.check(&request).await.is_none());
    }

    #[tokio::test]
    async fn test_deduplication_key_extraction() {
        let config = DeduplicationConfig::default();
        let dedup = RequestDeduplicator::new(config);

        // GET request should have a key
        let get_request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        assert!(dedup.extract_key(&get_request).is_some());

        // POST without idempotency key should not have a key
        let post_request = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        assert!(dedup.extract_key(&post_request).is_none());

        // POST with idempotency key should have a key
        let post_with_key = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .header("idempotency-key", "abc123")
            .body(Body::empty())
            .unwrap();

        assert!(dedup.extract_key(&post_with_key).is_some());
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let config = DeduplicationConfig::default();
        let dedup = RequestDeduplicator::new(config);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let result = CachedResult {
            status: 200,
            headers: vec![],
            body: bytes::Bytes::from("test response"),
        };

        dedup.store(&request, result).await;

        let cached = dedup.check(&request).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().status, 200);
    }
}
