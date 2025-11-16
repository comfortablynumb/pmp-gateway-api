pub mod hot_reload;
pub mod traffic_split;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use hot_reload::ConfigHotReload;
pub use traffic_split::{RoutingRule, TrafficSplitConfig, TrafficVariant};

/// Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Map of client ID to client configuration
    pub clients: HashMap<String, ClientConfig>,
    /// List of route configurations
    pub routes: Vec<RouteConfig>,
    /// Global server configuration
    #[serde(default)]
    pub server: ServerConfig,
}

/// Server-level configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// CORS configuration
    #[serde(default)]
    pub cors: Option<CorsConfig>,
    /// Request logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Global timeout in seconds
    #[serde(default = "default_global_timeout")]
    pub timeout: u64,
    /// Maximum request body size in bytes
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
    /// Security configuration
    #[serde(default)]
    pub security: SecurityConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            cors: None,
            logging: LoggingConfig::default(),
            timeout: default_global_timeout(),
            max_body_size: default_max_body_size(),
            rate_limit: None,
            security: SecurityConfig::default(),
        }
    }
}

/// CORS configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CorsConfig {
    /// Allowed origins (e.g., ["https://example.com", "*"])
    pub allowed_origins: Vec<String>,
    /// Allowed methods (e.g., ["GET", "POST"])
    #[serde(default = "default_cors_methods")]
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    #[serde(default)]
    pub allowed_headers: Vec<String>,
    /// Whether to allow credentials
    #[serde(default)]
    pub allow_credentials: bool,
    /// Max age for preflight cache in seconds
    #[serde(default = "default_cors_max_age")]
    pub max_age: u64,
}

/// Request logging configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    /// Log request bodies
    #[serde(default)]
    pub log_request_body: bool,
    /// Log response bodies
    #[serde(default)]
    pub log_response_body: bool,
    /// Log headers
    #[serde(default = "default_true")]
    pub log_headers: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_request_body: false,
            log_response_body: false,
            log_headers: true,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitConfig {
    /// Requests per second
    pub requests_per_second: u64,
    /// Burst size
    #[serde(default = "default_burst_size")]
    pub burst_size: u32,
}

/// Security configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SecurityConfig {
    /// API key validation
    #[serde(default)]
    pub api_keys: Option<ApiKeyConfig>,
    /// JWT validation
    #[serde(default)]
    pub jwt: Option<JwtConfig>,
    /// IP allowlist/blocklist
    #[serde(default)]
    pub ip_filter: Option<IpFilterConfig>,
}

/// API key configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiKeyConfig {
    /// Header name containing the API key
    #[serde(default = "default_api_key_header")]
    pub header: String,
    /// Valid API keys
    pub keys: Vec<String>,
}

/// JWT configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwtConfig {
    /// JWT secret or public key
    pub secret: String,
    /// Algorithm (HS256, RS256, etc.)
    #[serde(default = "default_jwt_algorithm")]
    pub algorithm: String,
    /// Whether to validate expiration
    #[serde(default = "default_true")]
    pub validate_exp: bool,
}

/// IP filter configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IpFilterConfig {
    /// IP allowlist (if set, only these IPs are allowed)
    #[serde(default)]
    pub allowlist: Vec<String>,
    /// IP blocklist (these IPs are blocked)
    #[serde(default)]
    pub blocklist: Vec<String>,
}

fn default_global_timeout() -> u64 {
    30
}

fn default_max_body_size() -> usize {
    10 * 1024 * 1024 // 10 MB
}

fn default_cors_methods() -> Vec<String> {
    vec![
        "GET".to_string(),
        "POST".to_string(),
        "PUT".to_string(),
        "DELETE".to_string(),
        "OPTIONS".to_string(),
    ]
}

fn default_cors_max_age() -> u64 {
    3600
}

fn default_burst_size() -> u32 {
    10
}

fn default_api_key_header() -> String {
    "x-api-key".to_string()
}

fn default_jwt_algorithm() -> String {
    "HS256".to_string()
}

fn default_true() -> bool {
    true
}

/// Client configuration (supports different client types)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClientConfig {
    Http(HttpClientConfig),
    Postgres(PostgresClientConfig),
    Mysql(MysqlClientConfig),
    Sqlite(SqliteClientConfig),
    Mongodb(MongodbClientConfig),
    Redis(RedisClientConfig),
}

/// HTTP client configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpClientConfig {
    /// Base URL for the HTTP client (if using a single backend)
    #[serde(default)]
    pub base_url: String,
    /// Multiple backend URLs (for load balancing)
    #[serde(default)]
    pub backends: Vec<String>,
    /// Load balancing strategy
    #[serde(default)]
    pub load_balance: Option<LoadBalanceStrategy>,
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
    /// Retry configuration
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    /// Circuit breaker configuration
    #[serde(default)]
    pub circuit_breaker: Option<CircuitBreakerConfigYaml>,
}

/// Load balancing strategy
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LoadBalanceStrategy {
    RoundRobin,
    Random,
    LeastConnections,
}

/// Retry configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Initial backoff in milliseconds
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_ms: u64,
    /// Maximum backoff in milliseconds
    #[serde(default = "default_max_backoff")]
    pub max_backoff_ms: u64,
}

fn default_max_retries() -> u32 {
    3
}

fn default_initial_backoff() -> u64 {
    100
}

fn default_max_backoff() -> u64 {
    5000
}

/// Circuit breaker configuration for YAML
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CircuitBreakerConfigYaml {
    /// Number of consecutive failures before opening the circuit
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    /// Timeout in seconds before attempting to close the circuit
    #[serde(default = "default_circuit_timeout")]
    pub timeout_seconds: u64,
}

fn default_failure_threshold() -> u32 {
    5
}

fn default_circuit_timeout() -> u64 {
    30
}

/// PostgreSQL client configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostgresClientConfig {
    /// Connection string (e.g., "postgres://user:pass@localhost/db")
    pub connection_string: String,
    /// Maximum number of connections in the pool
    #[serde(default = "default_max_connections_u32")]
    pub max_connections: u32,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// MySQL client configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MysqlClientConfig {
    /// Connection string (e.g., "mysql://user:pass@localhost/db")
    pub connection_string: String,
    /// Maximum number of connections in the pool
    #[serde(default = "default_max_connections_u32")]
    pub max_connections: u32,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// SQLite client configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SqliteClientConfig {
    /// Database file path (e.g., "sqlite://db.sqlite")
    pub database_path: String,
    /// Maximum number of connections in the pool
    #[serde(default = "default_max_connections_u32")]
    pub max_connections: u32,
}

/// MongoDB client configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MongodbClientConfig {
    /// Connection string (e.g., "mongodb://localhost:27017")
    pub connection_string: String,
    /// Database name
    pub database: String,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// Redis client configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisClientConfig {
    /// Connection string (e.g., "redis://localhost:6379")
    pub connection_string: String,
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

fn default_max_connections_u32() -> u32 {
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
    /// Optional response transformation
    #[serde(default)]
    pub response_transform: Option<ResponseTransform>,
    /// Execution mode: sequential or parallel (default: parallel)
    #[serde(default = "default_execution_mode")]
    pub execution_mode: ExecutionMode,
    /// Traffic split configuration for A/B testing or canary deployments
    #[serde(default)]
    pub traffic_split: Option<TrafficSplitConfig>,
    /// Traffic mirroring configuration for testing
    #[serde(default)]
    pub traffic_mirror: Option<crate::middleware::TrafficMirrorConfig>,
}

/// Execution mode for subrequests
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Execute all subrequests in parallel
    Parallel,
    /// Execute subrequests sequentially
    Sequential,
}

fn default_execution_mode() -> ExecutionMode {
    ExecutionMode::Parallel
}

/// Response transformation configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResponseTransform {
    /// JSONPath-like filter to extract specific data
    #[serde(default)]
    pub filter: Option<String>,
    /// Field mappings (rename fields in the response)
    #[serde(default)]
    pub field_mappings: HashMap<String, String>,
    /// Fields to include (if empty, include all)
    #[serde(default)]
    pub include_fields: Vec<String>,
    /// Fields to exclude
    #[serde(default)]
    pub exclude_fields: Vec<String>,
    /// Custom template for response transformation (supports interpolation)
    #[serde(default)]
    pub template: Option<String>,
}

/// Subrequest configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubrequestConfig {
    /// Optional name for this subrequest (used for referencing results)
    #[serde(default)]
    pub name: Option<String>,
    /// Reference to a client ID
    pub client_id: String,
    /// Optional condition for executing this subrequest
    #[serde(default)]
    pub condition: Option<Condition>,
    /// List of subrequest names this depends on (for sequential execution)
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Subrequest-specific configuration based on client type
    #[serde(flatten)]
    pub config: SubrequestTypeConfig,
}

/// Condition for conditional execution
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Condition {
    /// Always execute
    Always,
    /// Execute if a field exists
    FieldExists { field: String },
    /// Execute if a field equals a value
    FieldEquals { field: String, value: String },
    /// Execute if a field matches a regex
    FieldMatches { field: String, pattern: String },
    /// Execute if a header exists
    HeaderExists { header: String },
    /// Execute if a header equals a value
    HeaderEquals { header: String, value: String },
    /// Execute if query parameter exists
    QueryExists { param: String },
    /// Execute if query parameter equals a value
    QueryEquals { param: String, value: String },
    /// Combine multiple conditions with AND
    And { conditions: Vec<Condition> },
    /// Combine multiple conditions with OR
    Or { conditions: Vec<Condition> },
    /// Negate a condition
    Not { condition: Box<Condition> },
}

/// Type-specific subrequest configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SubrequestTypeConfig {
    Http(HttpSubrequestConfig),
    Postgres(SqlSubrequestConfig),
    Mysql(SqlSubrequestConfig),
    Sqlite(SqlSubrequestConfig),
    Mongodb(MongodbSubrequestConfig),
    Redis(RedisSubrequestConfig),
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

/// SQL subrequest configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SqlSubrequestConfig {
    /// SQL query to execute (supports interpolation)
    pub query: String,
    /// Query parameters (supports interpolation)
    #[serde(default)]
    pub params: Vec<String>,
}

/// MongoDB subrequest configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MongodbSubrequestConfig {
    /// Collection name
    pub collection: String,
    /// Operation type
    pub operation: MongoOperation,
}

/// MongoDB operation types
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum MongoOperation {
    Find {
        /// Filter (supports interpolation in JSON)
        filter: String,
        /// Optional limit
        #[serde(default)]
        limit: Option<i64>,
    },
    FindOne {
        /// Filter (supports interpolation in JSON)
        filter: String,
    },
    Insert {
        /// Document to insert (supports interpolation in JSON)
        document: String,
    },
    Update {
        /// Filter (supports interpolation in JSON)
        filter: String,
        /// Update document (supports interpolation in JSON)
        update: String,
    },
    Delete {
        /// Filter (supports interpolation in JSON)
        filter: String,
    },
}

/// Redis subrequest configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisSubrequestConfig {
    /// Redis operation
    pub operation: RedisOperation,
}

/// Redis operation types
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum RedisOperation {
    Get {
        /// Key (supports interpolation)
        key: String,
    },
    Set {
        /// Key (supports interpolation)
        key: String,
        /// Value (supports interpolation)
        value: String,
        /// Optional expiration in seconds
        #[serde(default)]
        expiration: Option<u64>,
    },
    Del {
        /// Key (supports interpolation)
        key: String,
    },
    Exists {
        /// Key (supports interpolation)
        key: String,
    },
    Hget {
        /// Key (supports interpolation)
        key: String,
        /// Field (supports interpolation)
        field: String,
    },
    Hset {
        /// Key (supports interpolation)
        key: String,
        /// Field (supports interpolation)
        field: String,
        /// Value (supports interpolation)
        value: String,
    },
}

fn default_method() -> String {
    "GET".to_string()
}

impl Config {
    /// Load configuration from a YAML file
    pub fn from_yaml_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        // Interpolate environment variables
        let interpolated = crate::env_interpolation::interpolate_yaml_string(&content);
        let config: Config = serde_yaml::from_str(&interpolated)?;
        Ok(config)
    }

    /// Load configuration with environment-specific overrides
    /// Tries to load base config, then overlays environment-specific config
    /// E.g., config.yaml + config.dev.yaml
    pub fn from_yaml_with_env(base_path: &str) -> anyhow::Result<Self> {
        // Load base configuration
        let mut config = Self::from_yaml_file(base_path)?;

        // Check for environment-specific config
        if let Ok(env) = std::env::var("ENV") {
            let env_path = base_path.replace(".yaml", &format!(".{}.yaml", env));
            if std::path::Path::new(&env_path).exists() {
                tracing::info!("Loading environment-specific config: {}", env_path);
                let env_config = Self::from_yaml_file(&env_path)?;
                // Merge configs - environment config takes precedence
                config = Self::merge_configs(config, env_config);
            }
        }

        config.validate()?;
        Ok(config)
    }

    /// Merge two configurations, with override taking precedence
    fn merge_configs(base: Config, override_cfg: Config) -> Config {
        Config {
            clients: {
                let mut merged = base.clients;
                merged.extend(override_cfg.clients);
                merged
            },
            routes: if override_cfg.routes.is_empty() {
                base.routes
            } else {
                override_cfg.routes
            },
            server: if override_cfg.server.cors.is_some()
                || override_cfg.server.rate_limit.is_some()
            {
                override_cfg.server
            } else {
                base.server
            },
        }
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

    #[test]
    fn test_sql_config_deserialization() {
        let yaml = r#"
clients:
  db1:
    type: postgres
    connection_string: "postgres://localhost/test"
    max_connections: 5

routes:
  - method: GET
    path: /users/:id
    subrequests:
      - client_id: db1
        type: postgres
        query: "SELECT * FROM users WHERE id = $1"
        params:
          - "${request.path.id}"
"#;

        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.clients.len(), 1);
        assert_eq!(config.routes.len(), 1);
    }

    #[test]
    fn test_conditional_execution() {
        let yaml = r#"
clients:
  api1:
    type: http
    base_url: "https://api.example.com"

routes:
  - method: GET
    path: /test
    subrequests:
      - client_id: api1
        type: http
        uri: /endpoint
        condition:
          type: headerexists
          header: "Authorization"
"#;

        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.routes[0].subrequests[0].condition.is_some());
    }
}
