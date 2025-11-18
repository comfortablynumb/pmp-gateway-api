#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// A/B testing and canary deployment configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrafficSplitConfig {
    /// Name of this traffic split
    pub name: String,
    /// Variants to route traffic to
    pub variants: Vec<TrafficVariant>,
    /// Rules for routing traffic
    #[serde(default)]
    pub rules: Vec<RoutingRule>,
}

/// A traffic variant (e.g., control vs experiment, or canary vs stable)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrafficVariant {
    /// Name of the variant (e.g., "control", "experiment", "canary")
    pub name: String,
    /// Client ID to route to
    pub client_id: String,
    /// Percentage of traffic (0-100)
    pub weight: u8,
    /// Whether this is a sticky variant (uses cookies to maintain consistency)
    #[serde(default)]
    pub sticky: bool,
}

/// Routing rule for traffic splitting
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RoutingRule {
    /// Route based on header value
    Header {
        name: String,
        value: String,
        variant: String,
    },
    /// Route based on cookie value
    Cookie {
        name: String,
        value: String,
        variant: String,
    },
    /// Route based on query parameter
    Query {
        name: String,
        value: String,
        variant: String,
    },
    /// Route based on percentage (0-100)
    Percentage { variant: String, percentage: u8 },
}

impl TrafficSplitConfig {
    /// Validate that weights sum to 100
    pub fn validate(&self) -> Result<(), String> {
        let total_weight: u32 = self.variants.iter().map(|v| v.weight as u32).sum();

        if total_weight != 100 {
            return Err(format!(
                "Traffic split '{}': variant weights must sum to 100, got {}",
                self.name, total_weight
            ));
        }

        // Check for duplicate variant names
        let mut names = std::collections::HashSet::new();
        for variant in &self.variants {
            if !names.insert(&variant.name) {
                return Err(format!(
                    "Traffic split '{}': duplicate variant name '{}'",
                    self.name, variant.name
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_split_validation_success() {
        let config = TrafficSplitConfig {
            name: "ab_test".to_string(),
            variants: vec![
                TrafficVariant {
                    name: "control".to_string(),
                    client_id: "backend_v1".to_string(),
                    weight: 50,
                    sticky: true,
                },
                TrafficVariant {
                    name: "experiment".to_string(),
                    client_id: "backend_v2".to_string(),
                    weight: 50,
                    sticky: true,
                },
            ],
            rules: vec![],
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_traffic_split_validation_weight_error() {
        let config = TrafficSplitConfig {
            name: "ab_test".to_string(),
            variants: vec![
                TrafficVariant {
                    name: "control".to_string(),
                    client_id: "backend_v1".to_string(),
                    weight: 60,
                    sticky: true,
                },
                TrafficVariant {
                    name: "experiment".to_string(),
                    client_id: "backend_v2".to_string(),
                    weight: 50,
                    sticky: true,
                },
            ],
            rules: vec![],
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_canary_deployment() {
        let config = TrafficSplitConfig {
            name: "canary".to_string(),
            variants: vec![
                TrafficVariant {
                    name: "stable".to_string(),
                    client_id: "backend_stable".to_string(),
                    weight: 90,
                    sticky: false,
                },
                TrafficVariant {
                    name: "canary".to_string(),
                    client_id: "backend_canary".to_string(),
                    weight: 10,
                    sticky: false,
                },
            ],
            rules: vec![],
        };

        assert!(config.validate().is_ok());
    }
}
