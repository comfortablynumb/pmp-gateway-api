use crate::clients::ClientManager;
use crate::conditions::evaluate_condition;
use crate::config::{
    Config, ExecutionMode, MongodbSubrequestConfig, RedisSubrequestConfig, SqlSubrequestConfig,
    SubrequestConfig, SubrequestTypeConfig,
};
use crate::interpolation::InterpolationContext;
use crate::transform::apply_transformation;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};

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
    let mut context = InterpolationContext::new(
        headers.clone(),
        path_params,
        query_params,
        Some(body),
        method.clone(),
    );

    // For demonstration, let's find the first route that matches the method
    // A more sophisticated implementation would do proper path matching
    if let Some(route_config) = state.config.routes.first() {
        let results = match route_config.execution_mode {
            ExecutionMode::Sequential => {
                execute_sequential(&state, &route_config.subrequests, &mut context).await?
            }
            ExecutionMode::Parallel => {
                execute_parallel(&state, &route_config.subrequests, &context).await?
            }
        };

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

/// Execute subrequests sequentially (allows data dependencies)
async fn execute_sequential(
    state: &AppState,
    subrequests: &[SubrequestConfig],
    context: &mut InterpolationContext,
) -> Result<Vec<Value>, AppError> {
    let mut results = Vec::new();

    for subrequest in subrequests {
        // Check condition if present
        if let Some(condition) = &subrequest.condition {
            if !evaluate_condition(condition, context) {
                debug!(
                    "Skipping subrequest {:?} - condition not met",
                    subrequest.name
                );
                continue;
            }
        }

        debug!(
            "Executing subrequest {:?} for client: {}",
            subrequest.name, subrequest.client_id
        );

        let result = execute_single_subrequest(state, subrequest, context).await?;

        // Store result in context if the subrequest has a name
        if let Some(name) = &subrequest.name {
            context.add_subrequest_result(name.clone(), result.clone());
        }

        results.push(result);
    }

    Ok(results)
}

/// Execute subrequests in parallel (for independent requests)
async fn execute_parallel(
    state: &AppState,
    subrequests: &[SubrequestConfig],
    context: &InterpolationContext,
) -> Result<Vec<Value>, AppError> {
    // Build dependency graph and execution order
    let execution_order = build_execution_order(subrequests)?;

    let mut all_results = Vec::new();
    let mut context_clone = context.clone();

    // Execute in waves based on dependencies
    for wave in execution_order {
        let mut wave_futures = Vec::new();

        for idx in wave {
            let subrequest = &subrequests[idx];

            // Check condition if present
            if let Some(condition) = &subrequest.condition {
                if !evaluate_condition(condition, &context_clone) {
                    debug!(
                        "Skipping subrequest {:?} - condition not met",
                        subrequest.name
                    );
                    continue;
                }
            }

            let state_clone = state.clone();
            let subrequest_clone = subrequest.clone();
            let context_for_task = context_clone.clone();

            wave_futures.push(async move {
                (
                    idx,
                    subrequest_clone.name.clone(),
                    execute_single_subrequest(&state_clone, &subrequest_clone, &context_for_task)
                        .await,
                )
            });
        }

        // Execute this wave in parallel
        let wave_results = futures::future::join_all(wave_futures).await;

        // Collect results and update context
        for (idx, name, result) in wave_results {
            match result {
                Ok(value) => {
                    if let Some(subreq_name) = name {
                        context_clone.add_subrequest_result(subreq_name, value.clone());
                    }
                    all_results.push((idx, value));
                }
                Err(e) => return Err(e),
            }
        }
    }

    // Sort results by original order
    all_results.sort_by_key(|(idx, _)| *idx);
    Ok(all_results.into_iter().map(|(_, v)| v).collect())
}

/// Build execution order based on dependencies
/// Returns waves of subrequest indices that can be executed in parallel
fn build_execution_order(subrequests: &[SubrequestConfig]) -> Result<Vec<Vec<usize>>, AppError> {
    let mut waves: Vec<Vec<usize>> = Vec::new();
    let mut executed: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Create name to index mapping
    let _name_to_idx: HashMap<String, usize> = subrequests
        .iter()
        .enumerate()
        .filter_map(|(idx, sr)| sr.name.as_ref().map(|name| (name.clone(), idx)))
        .collect();

    loop {
        let mut current_wave = Vec::new();

        for (idx, subrequest) in subrequests.iter().enumerate() {
            // Skip if already executed
            if let Some(name) = &subrequest.name {
                if executed.contains(name) {
                    continue;
                }
            } else if waves.iter().any(|wave| wave.contains(&idx)) {
                continue;
            }

            // Check if all dependencies are met
            let deps_met = subrequest
                .depends_on
                .iter()
                .all(|dep| executed.contains(dep));

            if deps_met {
                current_wave.push(idx);
                if let Some(name) = &subrequest.name {
                    executed.insert(name.clone());
                }
            }
        }

        if current_wave.is_empty() {
            break;
        }

        waves.push(current_wave);
    }

    // Check if all subrequests were scheduled
    if waves.iter().map(|w| w.len()).sum::<usize>() != subrequests.len() {
        return Err(AppError::CircularDependency);
    }

    Ok(waves)
}

/// Execute a single subrequest
async fn execute_single_subrequest(
    state: &AppState,
    subrequest: &SubrequestConfig,
    context: &InterpolationContext,
) -> Result<Value, AppError> {
    match &subrequest.config {
        SubrequestTypeConfig::Http(http_config) => {
            execute_http_subrequest(
                &state.client_manager,
                &subrequest.client_id,
                http_config,
                context,
            )
            .await
        }
        SubrequestTypeConfig::Postgres(sql_config)
        | SubrequestTypeConfig::Mysql(sql_config)
        | SubrequestTypeConfig::Sqlite(sql_config) => {
            execute_sql_subrequest(
                &state.client_manager,
                &subrequest.client_id,
                sql_config,
                context,
            )
            .await
        }
        SubrequestTypeConfig::Mongodb(mongo_config) => {
            execute_mongodb_subrequest(
                &state.client_manager,
                &subrequest.client_id,
                mongo_config,
                context,
            )
            .await
        }
        SubrequestTypeConfig::Redis(redis_config) => {
            execute_redis_subrequest(
                &state.client_manager,
                &subrequest.client_id,
                redis_config,
                context,
            )
            .await
        }
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
    #[allow(dead_code)]
    InvalidConfig(String),

    #[error("Route not found")]
    RouteNotFound,

    #[error("Circular dependency detected in subrequests")]
    CircularDependency,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::ClientNotFound(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::SubrequestFailed(ref msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::InvalidConfig(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::RouteNotFound => (StatusCode::NOT_FOUND, "Route not found".to_string()),
            AppError::CircularDependency => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Circular dependency detected in subrequests".to_string(),
            ),
        };

        error!("Request failed: {}", error_message);

        let body = json!({
            "error": error_message,
        });

        (status, axum::Json(body)).into_response()
    }
}
