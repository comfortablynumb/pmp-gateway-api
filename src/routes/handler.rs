use crate::clients::ClientManager;
use crate::conditions::evaluate_condition;
use crate::config::{Config, MongodbSubrequestConfig, RedisSubrequestConfig, SqlSubrequestConfig, SubrequestTypeConfig};
use crate::interpolation::InterpolationContext;
use crate::transform::apply_transformation;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Shared application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub client_manager: Arc<ClientManager>,
}

/// Generic route handler that processes subrequests
pub async fn handle_route(
    State(state): State<AppState>,
    method: Method,
    Path(path_params): Path<HashMap<String, String>>,
    Query(query_params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: String,
) -> Result<Response, AppError> {
    info!(
        "Handling request: {} with {} path params, {} query params",
        method,
        path_params.len(),
        query_params.len()
    );

    // Create interpolation context
    let context = InterpolationContext::new(
        headers.clone(),
        path_params,
        query_params,
        Some(body),
        method.clone(),
    );

    // Collect results from all subrequests
    let mut results = Vec::new();

    // For demonstration, let's find the first route that matches the method
    // A more sophisticated implementation would do proper path matching
    if let Some(route_config) = state.config.routes.first() {
        for subrequest in &route_config.subrequests {
            // Check condition if present
            if let Some(condition) = &subrequest.condition {
                if !evaluate_condition(condition, &context) {
                    debug!(
                        "Skipping subrequest for client {} - condition not met",
                        subrequest.client_id
                    );
                    continue;
                }
            }

            debug!("Executing subrequest for client: {}", subrequest.client_id);

            match &subrequest.config {
                SubrequestTypeConfig::Http(http_config) => {
                    let result = execute_http_subrequest(
                        &state.client_manager,
                        &subrequest.client_id,
                        http_config,
                        &context,
                    )
                    .await?;
                    results.push(result);
                }

                SubrequestTypeConfig::Postgres(sql_config)
                | SubrequestTypeConfig::Mysql(sql_config)
                | SubrequestTypeConfig::Sqlite(sql_config) => {
                    let result = execute_sql_subrequest(
                        &state.client_manager,
                        &subrequest.client_id,
                        sql_config,
                        &context,
                    )
                    .await?;
                    results.push(result);
                }

                SubrequestTypeConfig::Mongodb(mongo_config) => {
                    let result = execute_mongodb_subrequest(
                        &state.client_manager,
                        &subrequest.client_id,
                        mongo_config,
                        &context,
                    )
                    .await?;
                    results.push(result);
                }

                SubrequestTypeConfig::Redis(redis_config) => {
                    let result = execute_redis_subrequest(
                        &state.client_manager,
                        &subrequest.client_id,
                        redis_config,
                        &context,
                    )
                    .await?;
                    results.push(result);
                }
            }
        }

        // Apply response transformation if configured
        let mut response_data = json!({
            "subrequests": results,
            "count": results.len(),
        });

        if let Some(transform) = &route_config.response_transform {
            response_data = apply_transformation(response_data, transform, &context);
        }

        Ok((StatusCode::OK, axum::Json(response_data)).into_response())
    } else {
        Err(AppError::RouteNotFound)
    }
}

/// Execute an HTTP subrequest
async fn execute_http_subrequest(
    client_manager: &ClientManager,
    client_id: &str,
    config: &crate::config::HttpSubrequestConfig,
    context: &InterpolationContext,
) -> Result<serde_json::Value, AppError> {
    let client = client_manager
        .get_http_client(client_id)
        .ok_or_else(|| AppError::ClientNotFound(client_id.to_string()))?;

    // Interpolate URI
    let uri = context.interpolate(&config.uri);

    // Interpolate headers
    let headers: HashMap<String, String> = config
        .headers
        .iter()
        .map(|(k, v)| (k.clone(), context.interpolate(v)))
        .collect();

    // Interpolate body
    let body = config.body.as_ref().map(|b| context.interpolate(b));

    // Interpolate query params
    let query_params: HashMap<String, String> = config
        .query_params
        .iter()
        .map(|(k, v)| (k.clone(), context.interpolate(v)))
        .collect();

    // Execute the HTTP request
    let response = client
        .execute_request(&config.method, &uri, headers, body, query_params)
        .await
        .map_err(|e| AppError::SubrequestFailed(e.to_string()))?;

    Ok(json!({
        "client_id": client_id,
        "type": "http",
        "status": response.status,
        "body": response.body,
        "headers": response.headers,
    }))
}

/// Execute a SQL subrequest
async fn execute_sql_subrequest(
    client_manager: &ClientManager,
    client_id: &str,
    config: &SqlSubrequestConfig,
    context: &InterpolationContext,
) -> Result<serde_json::Value, AppError> {
    let client = client_manager
        .get_sql_client(client_id)
        .ok_or_else(|| AppError::ClientNotFound(client_id.to_string()))?;

    // Interpolate query
    let query = context.interpolate(&config.query);

    // Interpolate parameters
    let params: Vec<String> = config
        .params
        .iter()
        .map(|p| context.interpolate(p))
        .collect();

    // Execute the query
    let response = client
        .execute_query(&query, params)
        .await
        .map_err(|e| AppError::SubrequestFailed(e.to_string()))?;

    Ok(json!({
        "client_id": client_id,
        "type": "sql",
        "rows": response.rows,
        "row_count": response.row_count,
    }))
}

/// Execute a MongoDB subrequest
async fn execute_mongodb_subrequest(
    client_manager: &ClientManager,
    client_id: &str,
    config: &MongodbSubrequestConfig,
    context: &InterpolationContext,
) -> Result<serde_json::Value, AppError> {
    let client = client_manager
        .get_mongodb_client(client_id)
        .ok_or_else(|| AppError::ClientNotFound(client_id.to_string()))?;

    // Interpolate the operation's fields
    let interpolated_operation = interpolate_mongo_operation(&config.operation, context);

    // Execute the operation
    let response = client
        .execute_operation(&config.collection, &interpolated_operation)
        .await
        .map_err(|e| AppError::SubrequestFailed(e.to_string()))?;

    Ok(json!({
        "client_id": client_id,
        "type": "mongodb",
        "collection": config.collection,
        "operation": response.operation_type,
        "documents": response.documents,
        "count": response.count,
    }))
}

/// Execute a Redis subrequest
async fn execute_redis_subrequest(
    client_manager: &ClientManager,
    client_id: &str,
    config: &RedisSubrequestConfig,
    context: &InterpolationContext,
) -> Result<serde_json::Value, AppError> {
    let client = client_manager
        .get_redis_client(client_id)
        .ok_or_else(|| AppError::ClientNotFound(client_id.to_string()))?;

    // Interpolate the operation's fields
    let interpolated_operation = interpolate_redis_operation(&config.operation, context);

    // Execute the operation
    let response = client
        .execute_operation(&interpolated_operation)
        .await
        .map_err(|e| AppError::SubrequestFailed(e.to_string()))?;

    Ok(json!({
        "client_id": client_id,
        "type": "redis",
        "operation": response.operation_type,
        "value": response.value,
    }))
}

/// Interpolate MongoDB operation fields
fn interpolate_mongo_operation(
    operation: &crate::config::MongoOperation,
    context: &InterpolationContext,
) -> crate::config::MongoOperation {
    use crate::config::MongoOperation;

    match operation {
        MongoOperation::Find { filter, limit } => MongoOperation::Find {
            filter: context.interpolate(filter),
            limit: *limit,
        },
        MongoOperation::FindOne { filter } => MongoOperation::FindOne {
            filter: context.interpolate(filter),
        },
        MongoOperation::Insert { document } => MongoOperation::Insert {
            document: context.interpolate(document),
        },
        MongoOperation::Update { filter, update } => MongoOperation::Update {
            filter: context.interpolate(filter),
            update: context.interpolate(update),
        },
        MongoOperation::Delete { filter } => MongoOperation::Delete {
            filter: context.interpolate(filter),
        },
    }
}

/// Interpolate Redis operation fields
fn interpolate_redis_operation(
    operation: &crate::config::RedisOperation,
    context: &InterpolationContext,
) -> crate::config::RedisOperation {
    use crate::config::RedisOperation;

    match operation {
        RedisOperation::Get { key } => RedisOperation::Get {
            key: context.interpolate(key),
        },
        RedisOperation::Set {
            key,
            value,
            expiration,
        } => RedisOperation::Set {
            key: context.interpolate(key),
            value: context.interpolate(value),
            expiration: *expiration,
        },
        RedisOperation::Del { key } => RedisOperation::Del {
            key: context.interpolate(key),
        },
        RedisOperation::Exists { key } => RedisOperation::Exists {
            key: context.interpolate(key),
        },
        RedisOperation::Hget { key, field } => RedisOperation::Hget {
            key: context.interpolate(key),
            field: context.interpolate(field),
        },
        RedisOperation::Hset { key, field, value } => RedisOperation::Hset {
            key: context.interpolate(key),
            field: context.interpolate(field),
            value: context.interpolate(value),
        },
    }
}

/// Application error types
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Client not found: {0}")]
    ClientNotFound(String),

    #[error("Subrequest failed: {0}")]
    SubrequestFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Route not found")]
    RouteNotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::ClientNotFound(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::SubrequestFailed(ref msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::InvalidConfig(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::RouteNotFound => (StatusCode::NOT_FOUND, "Route not found".to_string()),
        };

        error!("Request failed: {}", error_message);

        let body = json!({
            "error": error_message,
        });

        (status, axum::Json(body)).into_response()
    }
}
