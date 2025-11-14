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
    Postgres(PostgresClientConfig),
    Mysql(MysqlClientConfig),
    Sqlite(SqliteClientConfig),
    Mongodb(MongodbClientConfig),
    Redis(RedisClientConfig),
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
    /// Reference to a client ID
    pub client_id: String,
    /// Optional condition for executing this subrequest
    #[serde(default)]
    pub condition: Option<Condition>,
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
        assert_eq!(config.routes[0].subrequests[0].condition.is_some(), true);
    }
}
