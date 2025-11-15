use axum::{
    body::Body,
    extract::{ConnectInfo, Request},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::config::{ApiKeyConfig, IpFilterConfig, JwtConfig, SecurityConfig};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

/// Security middleware that validates API keys, JWTs, and IP filters
pub async fn security_middleware(
    config: Arc<SecurityConfig>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Check IP filter
    if let Some(ref ip_filter) = config.ip_filter {
        if !is_ip_allowed(&addr.ip().to_string(), ip_filter) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(json!({"error": "IP address blocked"})),
            )
                .into_response());
        }
    }

    // Check API key
    if let Some(ref api_key_config) = config.api_keys {
        if !validate_api_key(request.headers(), api_key_config) {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid or missing API key"})),
            )
                .into_response());
        }
    }

    // Check JWT
    if let Some(ref jwt_config) = config.jwt {
        if !validate_jwt(request.headers(), jwt_config) {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid or missing JWT token"})),
            )
                .into_response());
        }
    }

    Ok(next.run(request).await)
}

fn is_ip_allowed(ip: &str, config: &IpFilterConfig) -> bool {
    // If allowlist is set, only those IPs are allowed
    if !config.allowlist.is_empty() {
        return config
            .allowlist
            .iter()
            .any(|allowed| ip.starts_with(allowed));
    }

    // Check blocklist
    if !config.blocklist.is_empty() {
        return !config
            .blocklist
            .iter()
            .any(|blocked| ip.starts_with(blocked));
    }

    true
}

fn validate_api_key(headers: &HeaderMap, config: &ApiKeyConfig) -> bool {
    if let Some(api_key) = headers.get(&config.header) {
        if let Ok(key_str) = api_key.to_str() {
            return config.keys.contains(&key_str.to_string());
        }
    }
    false
}

fn validate_jwt(headers: &HeaderMap, config: &JwtConfig) -> bool {
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                let algorithm = match config.algorithm.as_str() {
                    "HS256" => Algorithm::HS256,
                    "HS384" => Algorithm::HS384,
                    "HS512" => Algorithm::HS512,
                    "RS256" => Algorithm::RS256,
                    _ => Algorithm::HS256,
                };

                let mut validation = Validation::new(algorithm);
                validation.validate_exp = config.validate_exp;

                let key = DecodingKey::from_secret(config.secret.as_bytes());

                return decode::<Claims>(token, &key, &validation).is_ok();
            }
        }
    }
    false
}

/// Create security middleware with config
pub fn create_security_middleware(
    config: SecurityConfig,
) -> impl Fn(
    ConnectInfo<SocketAddr>,
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, Response>> + Send>>
       + Clone {
    let config = Arc::new(config);
    move |addr: ConnectInfo<SocketAddr>, request: Request, next: Next| {
        let config = config.clone();
        Box::pin(async move { security_middleware(config, addr, request, next).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_allowlist() {
        let config = IpFilterConfig {
            allowlist: vec!["192.168.1.".to_string()],
            blocklist: vec![],
        };

        assert!(is_ip_allowed("192.168.1.100", &config));
        assert!(!is_ip_allowed("10.0.0.1", &config));
    }

    #[test]
    fn test_ip_blocklist() {
        let config = IpFilterConfig {
            allowlist: vec![],
            blocklist: vec!["192.168.1.".to_string()],
        };

        assert!(!is_ip_allowed("192.168.1.100", &config));
        assert!(is_ip_allowed("10.0.0.1", &config));
    }

    #[test]
    fn test_no_ip_filter() {
        let config = IpFilterConfig {
            allowlist: vec![],
            blocklist: vec![],
        };

        assert!(is_ip_allowed("192.168.1.100", &config));
        assert!(is_ip_allowed("10.0.0.1", &config));
    }
}
