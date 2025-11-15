use crate::config::{MysqlClientConfig, PostgresClientConfig, SqliteClientConfig};
use anyhow::Result;
use serde_json::Value;
use sqlx::{Any, AnyPool, Column, Pool, Row, TypeInfo};
use tracing::{debug, info};

/// Generic SQL client that works with multiple database types
#[derive(Debug, Clone)]
pub struct SqlClient {
    pool: Pool<Any>,
    db_type: DatabaseType,
}

#[derive(Debug, Clone, Copy)]
pub enum DatabaseType {
    Postgres,
    Mysql,
    Sqlite,
}

impl SqlClient {
    /// Create a new PostgreSQL client
    pub async fn new_postgres(config: PostgresClientConfig) -> Result<Self> {
        info!(
            "Creating PostgreSQL client with max_connections={}",
            config.max_connections
        );

        let pool = AnyPool::connect_lazy(&config.connection_string)?;

        Ok(Self {
            pool,
            db_type: DatabaseType::Postgres,
        })
    }

    /// Create a new MySQL client
    pub async fn new_mysql(config: MysqlClientConfig) -> Result<Self> {
        info!(
            "Creating MySQL client with max_connections={}",
            config.max_connections
        );

        let pool = AnyPool::connect_lazy(&config.connection_string)?;

        Ok(Self {
            pool,
            db_type: DatabaseType::Mysql,
        })
    }

    /// Create a new SQLite client
    pub async fn new_sqlite(config: SqliteClientConfig) -> Result<Self> {
        info!("Creating SQLite client at {}", config.database_path);

        let pool = AnyPool::connect_lazy(&config.database_path)?;

        Ok(Self {
            pool,
            db_type: DatabaseType::Sqlite,
        })
    }

    /// Execute a query and return results as JSON
    pub async fn execute_query(&self, query: &str, params: Vec<String>) -> Result<SqlResponse> {
        debug!(
            "Executing {:?} query: {} with {} params",
            self.db_type,
            query,
            params.len()
        );

        // Build the query with parameters
        let mut query_builder = sqlx::query(query);
        for param in &params {
            query_builder = query_builder.bind(param);
        }

        // Execute query
        let rows = query_builder.fetch_all(&self.pool).await?;

        // Convert rows to JSON
        let mut results = Vec::new();
        for row in rows {
            let mut obj = serde_json::Map::new();

            for (i, column) in row.columns().iter().enumerate() {
                let column_name = column.name();
                let type_info = column.type_info();

                // Try to get the value as different types
                let value: Value = if type_info.name() == "TEXT" || type_info.name() == "VARCHAR" {
                    row.try_get::<String, _>(i)
                        .map(Value::String)
                        .unwrap_or(Value::Null)
                } else if type_info.name().contains("INT") {
                    row.try_get::<i64, _>(i)
                        .map(|v| Value::Number(v.into()))
                        .unwrap_or(Value::Null)
                } else if type_info.name().contains("BOOL") {
                    row.try_get::<bool, _>(i)
                        .map(Value::Bool)
                        .unwrap_or(Value::Null)
                } else {
                    // Fallback: try as string
                    row.try_get::<String, _>(i)
                        .map(Value::String)
                        .unwrap_or(Value::Null)
                };

                obj.insert(column_name.to_string(), value);
            }

            results.push(Value::Object(obj));
        }

        let row_count = results.len();
        Ok(SqlResponse {
            rows: results,
            row_count,
        })
    }

    /// Execute a non-query command (INSERT, UPDATE, DELETE)
    #[allow(dead_code)]
    pub async fn execute_command(&self, query: &str, params: Vec<String>) -> Result<SqlResponse> {
        debug!(
            "Executing {:?} command: {} with {} params",
            self.db_type,
            query,
            params.len()
        );

        let mut query_builder = sqlx::query(query);
        for param in &params {
            query_builder = query_builder.bind(param);
        }

        let result = query_builder.execute(&self.pool).await?;
        let rows_affected = result.rows_affected();

        Ok(SqlResponse {
            rows: vec![],
            row_count: rows_affected as usize,
        })
    }
}

/// SQL response structure
#[derive(Debug, Clone)]
pub struct SqlResponse {
    pub rows: Vec<Value>,
    pub row_count: usize,
}
