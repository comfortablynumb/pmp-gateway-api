#![allow(dead_code)]

use axum::{body::Body, extract::Request, http::HeaderValue, middleware::Next, response::Response};
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, trace};

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_capacity: u64,
    /// Time to live for cache entries
    pub ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 1000,
            ttl: Duration::from_secs(60),
        }
    }
}

/// Cache key based on request method and path
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct CacheKey {
    method: String,
    path: String,
}

/// Cached response data
#[derive(Debug, Clone)]
struct CachedResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: bytes::Bytes,
}

/// Response cache using moka
pub struct ResponseCache {
    cache: Cache<CacheKey, CachedResponse>,
}

impl ResponseCache {
    /// Create a new response cache
    pub fn new(config: CacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.ttl)
            .build();

        Self { cache }
    }

    /// Get a cached response
    async fn get(&self, key: &CacheKey) -> Option<CachedResponse> {
        self.cache.get(key).await
    }

    /// Store a response in the cache
    async fn put(&self, key: CacheKey, response: CachedResponse) {
        self.cache.insert(key, response).await;
    }
}

/// Create caching middleware
pub fn create_cache_middleware(
    cache: Arc<ResponseCache>,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone {
    move |request: Request, next: Next| {
        let cache = cache.clone();
        Box::pin(async move { cache_middleware(cache, request, next).await })
    }
}

/// Cache middleware handler
async fn cache_middleware(cache: Arc<ResponseCache>, request: Request, next: Next) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    // Only cache GET requests
    if method != "GET" {
        trace!("Skipping cache for non-GET request: {} {}", method, path);
        return next.run(request).await;
    }

    let key = CacheKey {
        method: method.clone(),
        path: path.clone(),
    };

    // Check cache
    if let Some(cached) = cache.get(&key).await {
        debug!("Cache HIT: {} {}", method, path);

        // Build response from cache
        let mut response = Response::builder().status(cached.status);

        // Add headers
        for (name, value) in &cached.headers {
            if let Ok(header_value) = value.parse::<HeaderValue>() {
                response = response.header(name, header_value);
            }
        }

        // Add cache hit header
        response = response.header("X-Cache", "HIT");

        return response
            .body(Body::from(cached.body.clone()))
            .unwrap_or_else(|_| Response::new(Body::empty()));
    }

    debug!("Cache MISS: {} {}", method, path);

    // Execute request
    let response = next.run(request).await;

    // Only cache successful responses (2xx)
    let status = response.status();
    if !status.is_success() {
        return response;
    }

    // Extract response parts for caching
    let (parts, body) = response.into_parts();

    // Read body
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            // Failed to read body, return error response
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

    let cached_response = CachedResponse {
        status: parts.status.as_u16(),
        headers: headers.clone(),
        body: body_bytes.clone(),
    };

    cache.put(key, cached_response).await;

    // Build response with body
    let mut response = Response::builder()
        .status(parts.status)
        .header("X-Cache", "MISS");

    for (name, value) in headers {
        if let Ok(header_value) = value.parse::<HeaderValue>() {
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

    #[tokio::test]
    async fn test_cache_creation() {
        let config = CacheConfig::default();
        let cache = ResponseCache::new(config);

        // Verify cache is empty
        let key = CacheKey {
            method: "GET".to_string(),
            path: "/test".to_string(),
        };

        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_put_get() {
        let config = CacheConfig::default();
        let cache = ResponseCache::new(config);

        let key = CacheKey {
            method: "GET".to_string(),
            path: "/test".to_string(),
        };

        let response = CachedResponse {
            status: 200,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: bytes::Bytes::from("test response"),
        };

        cache.put(key.clone(), response.clone()).await;

        let cached = cache.get(&key).await;
        assert!(cached.is_some());

        let cached = cached.unwrap();
        assert_eq!(cached.status, 200);
        assert_eq!(cached.body, bytes::Bytes::from("test response"));
    }
}
