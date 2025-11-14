use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Map of client ID to client configuration
    pub clients: HashMap<String, ClientConfig>,
    /// List of route configurations
    pub routes: Vec<RouteConfig>,
}

/// Client configuration (supports different client types)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClientConfig {
    Http(HttpClientConfig),
    // Future: Sql(SqlClientConfig),
    // Future: Redis(RedisClientConfig),
}

/// HTTP client configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpClientConfig {
    /// Base URL for the HTTP client
    pub base_url: String,
    /// Default headers to include in all requests
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Minimum number of connections in the pool
    #[serde(default = "default_min_connections")]
    pub min_connections: usize,
    /// Maximum number of connections in the pool
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_min_connections() -> usize {
    1
}

fn default_max_connections() -> usize {
    10
}

fn default_timeout() -> u64 {
    30
}

/// Route configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouteConfig {
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    /// URI path for this route
    pub path: String,
    /// List of subrequests to execute for this route
    pub subrequests: Vec<SubrequestConfig>,
}

/// Subrequest configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubrequestConfig {
    /// Reference to a client ID
    pub client_id: String,
    /// Subrequest-specific configuration based on client type
    #[serde(flatten)]
    pub config: SubrequestTypeConfig,
}

/// Type-specific subrequest configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SubrequestTypeConfig {
    Http(HttpSubrequestConfig),
    // Future: Sql(SqlSubrequestConfig),
    // Future: Redis(RedisSubrequestConfig),
}

/// HTTP subrequest configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpSubrequestConfig {
    /// URI to append to the client's base_url
    pub uri: String,
    /// HTTP method for this subrequest (GET, POST, etc.)
    #[serde(default = "default_method")]
    pub method: String,
    /// Additional headers for this specific subrequest
    /// Supports interpolation like: ${request.headers["Authorization"]}
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body template (supports interpolation)
    #[serde(default)]
    pub body: Option<String>,
    /// Query parameters (supports interpolation)
    #[serde(default)]
    pub query_params: HashMap<String, String>,
}

fn default_method() -> String {
    "GET".to_string()
}

impl Config {
    /// Load configuration from a YAML file
    pub fn from_yaml_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate that all client_ids in subrequests exist
        for route in &self.routes {
            for subrequest in &route.subrequests {
                if !self.clients.contains_key(&subrequest.client_id) {
                    anyhow::bail!(
                        "Route {} references unknown client_id: {}",
                        route.path,
                        subrequest.client_id
                    );
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_deserialization() {
        let yaml = r#"
clients:
  api1:
    type: http
    base_url: "https://api.example.com"
    headers:
      User-Agent: "PMP-Gateway/1.0"
    min_connections: 2
    max_connections: 20
    timeout: 60

routes:
  - method: GET
    path: /api/users/:id
    subrequests:
      - client_id: api1
        type: http
        uri: /users/${request.path.id}
        method: GET
        headers:
          Authorization: "${request.headers[\"Authorization\"]}"
"#;

        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.clients.len(), 1);
        assert_eq!(config.routes.len(), 1);
    }
}
