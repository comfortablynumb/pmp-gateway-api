use crate::config::{RoutingRule, TrafficSplitConfig, TrafficVariant};
use axum::extract::Request;
use tracing::debug;

/// Traffic selector for A/B testing and canary deployments
pub struct TrafficSelector {
    config: TrafficSplitConfig,
}

impl TrafficSelector {
    /// Create a new traffic selector
    pub fn new(config: TrafficSplitConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Select a variant based on request properties
    pub fn select_variant(
        &self,
        request: &Request,
        sticky_cookie: Option<&str>,
    ) -> &TrafficVariant {
        // Check if there's a sticky cookie
        if let Some(cookie_variant) = sticky_cookie {
            if let Some(variant) = self
                .config
                .variants
                .iter()
                .find(|v| v.name == cookie_variant)
            {
                debug!("Using sticky variant from cookie: {}", cookie_variant);
                return variant;
            }
        }

        // Check rules in order
        for rule in &self.config.rules {
            if let Some(variant) = self.check_rule(rule, request) {
                debug!("Matched rule, using variant: {}", variant.name);
                return variant;
            }
        }

        // Fall back to weighted selection
        self.select_weighted_variant(request)
    }

    /// Check a routing rule
    fn check_rule(&self, rule: &RoutingRule, request: &Request) -> Option<&TrafficVariant> {
        match rule {
            RoutingRule::Header {
                name,
                value,
                variant,
            } => {
                if let Some(header_value) = request.headers().get(name) {
                    if header_value.to_str().ok()? == value {
                        return self.find_variant(variant);
                    }
                }
            }
            RoutingRule::Cookie {
                name,
                value,
                variant,
            } => {
                if let Some(cookie_header) = request.headers().get("cookie") {
                    if let Ok(cookie_str) = cookie_header.to_str() {
                        if parse_cookie(cookie_str, name) == Some(value.as_str()) {
                            return self.find_variant(variant);
                        }
                    }
                }
            }
            RoutingRule::Query {
                name,
                value,
                variant,
            } => {
                if let Some(query) = request.uri().query() {
                    if parse_query_param(query, name) == Some(value.as_str()) {
                        return self.find_variant(variant);
                    }
                }
            }
            RoutingRule::Percentage { .. } => {
                // Percentage rules are handled in weighted selection
            }
        }
        None
    }

    /// Select variant based on weights
    fn select_weighted_variant(&self, request: &Request) -> &TrafficVariant {
        // Use request path + method as seed for consistent hashing
        let seed = format!("{}{}", request.method(), request.uri().path());
        let hash = simple_hash(&seed);
        let mut cumulative = 0u32;
        let bucket = hash % 100;

        for variant in &self.config.variants {
            cumulative += variant.weight as u32;
            if bucket < cumulative {
                debug!(
                    "Selected variant '{}' (bucket: {}, weight: {})",
                    variant.name, bucket, variant.weight
                );
                return variant;
            }
        }

        // Fallback to first variant (should never happen if weights sum to 100)
        &self.config.variants[0]
    }

    /// Find variant by name
    fn find_variant(&self, name: &str) -> Option<&TrafficVariant> {
        self.config.variants.iter().find(|v| v.name == name)
    }

    /// Get the configuration
    pub fn config(&self) -> &TrafficSplitConfig {
        &self.config
    }
}

/// Simple hash function for consistent variant selection
fn simple_hash(s: &str) -> u32 {
    s.bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32))
}

/// Parse a cookie value
fn parse_cookie<'a>(cookie_str: &'a str, name: &str) -> Option<&'a str> {
    for part in cookie_str.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            if k == name {
                return Some(v);
            }
        }
    }
    None
}

/// Parse a query parameter
fn parse_query_param<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    for part in query.split('&') {
        if let Some((k, v)) = part.split_once('=') {
            if k == name {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Method;

    #[test]
    fn test_simple_hash_consistency() {
        let hash1 = simple_hash("test");
        let hash2 = simple_hash("test");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_parse_cookie() {
        let cookie_str = "session=abc123; variant=experiment; path=/";
        assert_eq!(parse_cookie(cookie_str, "variant"), Some("experiment"));
        assert_eq!(parse_cookie(cookie_str, "session"), Some("abc123"));
        assert_eq!(parse_cookie(cookie_str, "missing"), None);
    }

    #[test]
    fn test_parse_query_param() {
        let query = "variant=experiment&user_id=123";
        assert_eq!(parse_query_param(query, "variant"), Some("experiment"));
        assert_eq!(parse_query_param(query, "user_id"), Some("123"));
        assert_eq!(parse_query_param(query, "missing"), None);
    }

    #[test]
    fn test_weighted_selection() {
        let config = TrafficSplitConfig {
            name: "test".to_string(),
            variants: vec![
                TrafficVariant {
                    name: "a".to_string(),
                    client_id: "backend_a".to_string(),
                    weight: 50,
                    sticky: false,
                },
                TrafficVariant {
                    name: "b".to_string(),
                    client_id: "backend_b".to_string(),
                    weight: 50,
                    sticky: false,
                },
            ],
            rules: vec![],
        };

        let selector = TrafficSelector::new(config).unwrap();

        // Create a test request
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let variant = selector.select_variant(&request, None);
        assert!(variant.name == "a" || variant.name == "b");
    }
}
