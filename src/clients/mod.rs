pub mod http;
pub mod load_balancer;
pub mod mongodb;
pub mod redis_client;
pub mod sql;

use crate::config::{ClientConfig, Config};
use anyhow::Result;
use std::collections::HashMap;

pub use http::HttpClient;
pub use load_balancer::LoadBalancer;
pub use mongodb::MongodbClient;
pub use redis_client::RedisClient;
pub use sql::SqlClient;

/// Client manager that holds all configured clients
#[derive(Debug, Clone)]
pub struct ClientManager {
    http_clients: HashMap<String, HttpClient>,
    sql_clients: HashMap<String, SqlClient>,
    mongodb_clients: HashMap<String, MongodbClient>,
    redis_clients: HashMap<String, RedisClient>,
}

impl ClientManager {
    /// Create a new client manager from configuration
    pub async fn from_config(config: &Config) -> Result<Self> {
        let mut http_clients = HashMap::new();
        let mut sql_clients = HashMap::new();
        let mut mongodb_clients = HashMap::new();
        let mut redis_clients = HashMap::new();

        for (client_id, client_config) in &config.clients {
            match client_config {
                ClientConfig::Http(http_config) => {
                    let client = HttpClient::new(http_config.clone())?;
                    http_clients.insert(client_id.clone(), client);
                }
                ClientConfig::Postgres(pg_config) => {
                    let client = SqlClient::new_postgres(pg_config.clone()).await?;
                    sql_clients.insert(client_id.clone(), client);
                }
                ClientConfig::Mysql(mysql_config) => {
                    let client = SqlClient::new_mysql(mysql_config.clone()).await?;
                    sql_clients.insert(client_id.clone(), client);
                }
                ClientConfig::Sqlite(sqlite_config) => {
                    let client = SqlClient::new_sqlite(sqlite_config.clone()).await?;
                    sql_clients.insert(client_id.clone(), client);
                }
                ClientConfig::Mongodb(mongo_config) => {
                    let client = MongodbClient::new(mongo_config.clone()).await?;
                    mongodb_clients.insert(client_id.clone(), client);
                }
                ClientConfig::Redis(redis_config) => {
                    let client = RedisClient::new(redis_config.clone()).await?;
                    redis_clients.insert(client_id.clone(), client);
                }
            }
        }

        Ok(Self {
            http_clients,
            sql_clients,
            mongodb_clients,
            redis_clients,
        })
    }

    /// Get an HTTP client by ID
    pub fn get_http_client(&self, client_id: &str) -> Option<&HttpClient> {
        self.http_clients.get(client_id)
    }

    /// Get a SQL client by ID
    pub fn get_sql_client(&self, client_id: &str) -> Option<&SqlClient> {
        self.sql_clients.get(client_id)
    }

    /// Get a MongoDB client by ID
    pub fn get_mongodb_client(&self, client_id: &str) -> Option<&MongodbClient> {
        self.mongodb_clients.get(client_id)
    }

    /// Get a Redis client by ID
    pub fn get_redis_client(&self, client_id: &str) -> Option<&RedisClient> {
        self.redis_clients.get(client_id)
    }
}
