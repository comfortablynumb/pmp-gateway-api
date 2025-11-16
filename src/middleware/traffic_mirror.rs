use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Traffic mirroring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficMirrorConfig {
    /// Mirror backend URL
    pub mirror_url: String,
    /// Percentage of traffic to mirror (0-100)
    pub sample_rate: u8,
    /// Timeout for mirror requests in seconds
    pub timeout: u64,
    /// Whether to wait for mirror response (usually false)
    pub blocking: bool,
}

impl TrafficMirrorConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.sample_rate > 100 {
            return Err("Sample rate must be between 0 and 100".to_string());
        }
        if self.timeout == 0 {
            return Err("Timeout must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// Traffic mirror state
#[derive(Clone)]
pub struct TrafficMirror {
    config: TrafficMirrorConfig,
    client: Client,
}

impl TrafficMirror {
    /// Create a new traffic mirror
    pub fn new(config: TrafficMirrorConfig) -> Result<Self, String> {
        config.validate()?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        info!(
            "Traffic mirror configured: {} (sample rate: {}%)",
            config.mirror_url, config.sample_rate
        );

        Ok(Self { config, client })
    }

    /// Check if request should be mirrored based on sample rate
    fn should_mirror(&self, request: &Request) -> bool {
        if self.config.sample_rate == 0 {
            return false;
        }
        if self.config.sample_rate >= 100 {
            return true;
        }

        // Use request path for consistent sampling
        let path = request.uri().path();
        let hash = simple_hash(path);
        (hash % 100) < self.config.sample_rate as u32
    }

    /// Mirror a request to the configured backend
    async fn mirror_request(&self, request: &Request) {
        let method_str = request.method().as_str().to_string();
        let path = request.uri().path().to_string();
        let query = request.uri().query().map(|q| q.to_string());

        // Extract headers as Vec of tuples
        let headers: Vec<(String, String)> = request
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                let name_str = name.as_str();
                if name_str != "host" && name_str != "content-length" {
                    value.to_str().ok().map(|v| (name_str.to_string(), v.to_string()))
                } else {
                    None
                }
            })
            .collect();

        self.send_mirror_request(method_str, path, query, headers).await;
    }

    /// Send mirror request with primitive types
    async fn send_mirror_request(
        &self,
        method_str: String,
        path: String,
        query: Option<String>,
        headers: Vec<(String, String)>,
    ) {
        // Build mirror URL
        let mirror_url = format!(
            "{}{}{}",
            self.config.mirror_url,
            path,
            query.map(|q| format!("?{}", q)).unwrap_or_default()
        );

        debug!("Mirroring {} {} to {}", method_str, path, mirror_url);

        // Convert to reqwest Method
        let reqwest_method = match method_str.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            "HEAD" => reqwest::Method::HEAD,
            "OPTIONS" => reqwest::Method::OPTIONS,
            _ => {
                error!("Unsupported method for mirroring: {}", method_str);
                return;
            }
        };

        // Create mirror request
        let mut mirror_req = self.client.request(reqwest_method, &mirror_url);

        // Add headers
        for (name, value) in headers {
            mirror_req = mirror_req.header(name, value);
        }

        // Add mirror identification header
        mirror_req = mirror_req.header("X-Traffic-Mirror", "true");

        // Send mirror request
        match mirror_req.send().await {
            Ok(resp) => {
                debug!(
                    "Mirror request completed: {} {} -> {}",
                    method_str, path, resp.status()
                );
            }
            Err(e) => {
                error!("Mirror request failed: {} {} -> {}", method_str, path, e);
            }
        }
    }
}

/// Simple hash function for consistent sampling
fn simple_hash(s: &str) -> u32 {
    s.bytes().fold(0u32, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as u32)
    })
}

/// Create traffic mirroring middleware
pub fn create_traffic_mirror_middleware(
    config: TrafficMirrorConfig,
) -> Result<Arc<TrafficMirror>, String> {
    TrafficMirror::new(config).map(Arc::new)
}

/// Traffic mirroring middleware
pub async fn traffic_mirror_middleware(
    State(mirror): State<Arc<TrafficMirror>>,
    request: Request,
    next: Next,
) -> Response {
    // Check if we should mirror this request
    let should_mirror = mirror.should_mirror(&request);

    if should_mirror {
        // Extract request data for mirroring
        let method_str = request.method().as_str().to_string();
        let path = request.uri().path().to_string();
        let query = request.uri().query().map(|q| q.to_string());
        let headers: Vec<(String, String)> = request
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                let name_str = name.as_str();
                if name_str != "host" && name_str != "content-length" {
                    value.to_str().ok().map(|v| (name_str.to_string(), v.to_string()))
                } else {
                    None
                }
            })
            .collect();

        if mirror.config.blocking {
            // Wait for mirror request (rare, usually for testing)
            mirror.send_mirror_request(method_str, path, query, headers).await;
        } else {
            // Fire and forget (common case)
            let mirror_clone = mirror.clone();
            tokio::spawn(async move {
                mirror_clone.send_mirror_request(method_str, path, query, headers).await;
            });
        }
    }

    // Continue with primary request
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, Uri};

    #[test]
    fn test_config_validation() {
        let valid_config = TrafficMirrorConfig {
            mirror_url: "http://test.com".to_string(),
            sample_rate: 50,
            timeout: 5,
            blocking: false,
        };
        assert!(valid_config.validate().is_ok());

        let invalid_sample = TrafficMirrorConfig {
            mirror_url: "http://test.com".to_string(),
            sample_rate: 101,
            timeout: 5,
            blocking: false,
        };
        assert!(invalid_sample.validate().is_err());

        let invalid_timeout = TrafficMirrorConfig {
            mirror_url: "http://test.com".to_string(),
            sample_rate: 50,
            timeout: 0,
            blocking: false,
        };
        assert!(invalid_timeout.validate().is_err());
    }

    #[test]
    fn test_simple_hash_consistency() {
        let hash1 = simple_hash("/test/path");
        let hash2 = simple_hash("/test/path");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_should_mirror_sample_rate() {
        let config_0 = TrafficMirrorConfig {
            mirror_url: "http://test.com".to_string(),
            sample_rate: 0,
            timeout: 5,
            blocking: false,
        };
        let mirror_0 = TrafficMirror::new(config_0).unwrap();

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        assert!(!mirror_0.should_mirror(&request));

        let config_100 = TrafficMirrorConfig {
            mirror_url: "http://test.com".to_string(),
            sample_rate: 100,
            timeout: 5,
            blocking: false,
        };
        let mirror_100 = TrafficMirror::new(config_100).unwrap();
        assert!(mirror_100.should_mirror(&request));
    }

    #[test]
    fn test_create_traffic_mirror() {
        let config = TrafficMirrorConfig {
            mirror_url: "http://localhost:8081".to_string(),
            sample_rate: 10,
            timeout: 3,
            blocking: false,
        };

        let result = create_traffic_mirror_middleware(config);
        assert!(result.is_ok());
    }
}
