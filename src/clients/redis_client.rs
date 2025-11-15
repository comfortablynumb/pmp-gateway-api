use crate::config::{RedisClientConfig, RedisOperation};
use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};
use serde_json::Value;
use tracing::{debug, info};

/// Redis client
#[derive(Clone)]
pub struct RedisClient {
    manager: ConnectionManager,
}

impl std::fmt::Debug for RedisClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisClient").finish()
    }
}

impl RedisClient {
    /// Create a new Redis client
    pub async fn new(config: RedisClientConfig) -> Result<Self> {
        info!("Creating Redis client");

        let client = Client::open(config.connection_string.as_str())?;
        let manager = ConnectionManager::new(client).await?;

        Ok(Self { manager })
    }

    /// Execute a Redis operation
    pub async fn execute_operation(&self, operation: &RedisOperation) -> Result<RedisResponse> {
        debug!("Executing Redis operation");

        let mut conn = self.manager.clone();

        match operation {
            RedisOperation::Get { key } => {
                let value: Option<String> = conn.get(key).await?;

                Ok(RedisResponse {
                    value: value.map(Value::String),
                    operation_type: "get".to_string(),
                })
            }

            RedisOperation::Set {
                key,
                value,
                expiration,
            } => {
                if let Some(exp) = expiration {
                    let _: () = conn.set_ex(key, value, *exp).await?;
                } else {
                    let _: () = conn.set(key, value).await?;
                }

                Ok(RedisResponse {
                    value: Some(Value::String("OK".to_string())),
                    operation_type: "set".to_string(),
                })
            }

            RedisOperation::Del { key } => {
                let deleted: i32 = conn.del(key).await?;

                Ok(RedisResponse {
                    value: Some(Value::Number(deleted.into())),
                    operation_type: "del".to_string(),
                })
            }

            RedisOperation::Exists { key } => {
                let exists: bool = conn.exists(key).await?;

                Ok(RedisResponse {
                    value: Some(Value::Bool(exists)),
                    operation_type: "exists".to_string(),
                })
            }

            RedisOperation::Hget { key, field } => {
                let value: Option<String> = conn.hget(key, field).await?;

                Ok(RedisResponse {
                    value: value.map(Value::String),
                    operation_type: "hget".to_string(),
                })
            }

            RedisOperation::Hset { key, field, value } => {
                let _: () = conn.hset(key, field, value).await?;

                Ok(RedisResponse {
                    value: Some(Value::String("OK".to_string())),
                    operation_type: "hset".to_string(),
                })
            }
        }
    }
}

/// Redis response structure
#[derive(Debug, Clone)]
pub struct RedisResponse {
    pub value: Option<Value>,
    pub operation_type: String,
}
