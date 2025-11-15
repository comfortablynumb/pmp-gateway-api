use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use serde_json::json;
use std::num::NonZeroU32;
use std::sync::Arc;

use crate::config::RateLimitConfig;

pub type AppRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Create a rate limiter from config
pub fn create_rate_limiter(config: &RateLimitConfig) -> AppRateLimiter {
    let quota = Quota::per_second(
        NonZeroU32::new(config.requests_per_second.try_into().unwrap_or(10)).unwrap(),
    )
    .allow_burst(NonZeroU32::new(config.burst_size).unwrap());

    Arc::new(RateLimiter::direct(quota))
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    limiter: AppRateLimiter,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    match limiter.check() {
        Ok(_) => Ok(next.run(request).await),
        Err(_) => Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error": "Rate limit exceeded"})),
        )
            .into_response()),
    }
}

/// Create rate limiting middleware with limiter
pub fn create_rate_limit_middleware(
    limiter: AppRateLimiter,
) -> impl Fn(
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, Response>> + Send>>
       + Clone {
    move |request: Request, next: Next| {
        let limiter = limiter.clone();
        Box::pin(async move { rate_limit_middleware(limiter, request, next).await })
    }
}
