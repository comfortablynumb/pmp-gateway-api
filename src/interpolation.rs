use axum::http::{HeaderMap, Method};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

static INTERPOLATION_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_interpolation_regex() -> &'static Regex {
    INTERPOLATION_REGEX.get_or_init(|| {
        Regex::new(r"\$\{([^}]+)\}").expect("Failed to compile interpolation regex")
    })
}

/// Context for interpolation containing request data
#[derive(Debug, Clone)]
pub struct InterpolationContext {
    pub headers: HeaderMap,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: Option<String>,
    pub method: Method,
    /// Results from previously executed subrequests (name -> result JSON)
    pub subrequest_results: HashMap<String, Value>,
}

impl InterpolationContext {
    pub fn new(
        headers: HeaderMap,
        path_params: HashMap<String, String>,
        query_params: HashMap<String, String>,
        body: Option<String>,
        method: Method,
    ) -> Self {
        Self {
            headers,
            path_params,
            query_params,
            body,
            method,
            subrequest_results: HashMap::new(),
        }
    }

    /// Add a subrequest result to the context
    pub fn add_subrequest_result(&mut self, name: String, result: Value) {
        self.subrequest_results.insert(name, result);
    }

    /// Interpolate a template string with request data
    /// Supports:
    /// - ${request.headers["Header-Name"]}
    /// - ${request.path.param_name}
    /// - ${request.query.param_name}
    /// - ${request.body}
    /// - ${request.method}
    /// - ${subrequest.name.field.path} (access previous subrequest results)
    pub fn interpolate(&self, template: &str) -> String {
        let regex = get_interpolation_regex();

        regex.replace_all(template, |caps: &regex::Captures| {
            let expr = &caps[1];
            self.evaluate_expression(expr)
        }).to_string()
    }

    fn evaluate_expression(&self, expr: &str) -> String {
        let expr = expr.trim();

        // Handle request.headers["Header-Name"]
        if let Some(header_expr) = expr.strip_prefix("request.headers[") {
            if let Some(header_name) = header_expr.strip_suffix(']') {
                let header_name = header_name.trim_matches('"').trim_matches('\'');
                return self.headers
                    .get(header_name)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
            }
        }

        // Handle request.path.param_name
        if let Some(param_name) = expr.strip_prefix("request.path.") {
            return self.path_params
                .get(param_name)
                .cloned()
                .unwrap_or_default();
        }

        // Handle request.query.param_name
        if let Some(param_name) = expr.strip_prefix("request.query.") {
            return self.query_params
                .get(param_name)
                .cloned()
                .unwrap_or_default();
        }

        // Handle request.body
        if expr == "request.body" {
            return self.body.clone().unwrap_or_default();
        }

        // Handle request.method
        if expr == "request.method" {
            return self.method.as_str().to_string();
        }

        // Handle subrequest.name.path (access previous subrequest results)
        if let Some(subreq_expr) = expr.strip_prefix("subrequest.") {
            return self.extract_subrequest_value(subreq_expr);
        }

        // If no match, return the original expression
        format!("${{{}}}", expr)
    }

    /// Extract a value from a subrequest result using dot notation
    /// Examples: "user_request.body.id", "api_call.status"
    fn extract_subrequest_value(&self, path: &str) -> String {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return String::new();
        }

        // First part is the subrequest name
        let subreq_name = parts[0];

        if let Some(result) = self.subrequest_results.get(subreq_name) {
            if parts.len() == 1 {
                // Return the whole result as JSON string
                return serde_json::to_string(result).unwrap_or_default();
            }

            // Navigate through the JSON path
            let mut current = result.clone();
            for part in &parts[1..] {
                current = match current {
                    Value::Object(map) => map.get(*part).cloned().unwrap_or(Value::Null),
                    Value::Array(arr) => {
                        // Try to parse as array index
                        if let Ok(index) = part.parse::<usize>() {
                            arr.get(index).cloned().unwrap_or(Value::Null)
                        } else {
                            Value::Null
                        }
                    }
                    _ => Value::Null,
                };
            }

            // Convert the final value to string
            match current {
                Value::String(s) => s,
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => String::new(),
                _ => serde_json::to_string(&current).unwrap_or_default(),
            }
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue, Method};

    #[test]
    fn test_header_interpolation() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer token123"));

        let ctx = InterpolationContext::new(
            headers,
            HashMap::new(),
            HashMap::new(),
            None,
            Method::GET,
        );

        let result = ctx.interpolate("Authorization: ${request.headers[\"authorization\"]}");
        assert_eq!(result, "Authorization: Bearer token123");
    }

    #[test]
    fn test_path_param_interpolation() {
        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), "123".to_string());

        let ctx = InterpolationContext::new(
            HeaderMap::new(),
            path_params,
            HashMap::new(),
            None,
            Method::GET,
        );

        let result = ctx.interpolate("/users/${request.path.id}");
        assert_eq!(result, "/users/123");
    }

    #[test]
    fn test_query_param_interpolation() {
        let mut query_params = HashMap::new();
        query_params.insert("filter".to_string(), "active".to_string());

        let ctx = InterpolationContext::new(
            HeaderMap::new(),
            HashMap::new(),
            query_params,
            None,
            Method::GET,
        );

        let result = ctx.interpolate("status=${request.query.filter}");
        assert_eq!(result, "status=active");
    }

    #[test]
    fn test_body_interpolation() {
        let ctx = InterpolationContext::new(
            HeaderMap::new(),
            HashMap::new(),
            HashMap::new(),
            Some(r#"{"key":"value"}"#.to_string()),
            Method::POST,
        );

        let result = ctx.interpolate("Body: ${request.body}");
        assert_eq!(result, r#"Body: {"key":"value"}"#);
    }

    #[test]
    fn test_method_interpolation() {
        let ctx = InterpolationContext::new(
            HeaderMap::new(),
            HashMap::new(),
            HashMap::new(),
            None,
            Method::POST,
        );

        let result = ctx.interpolate("Method is ${request.method}");
        assert_eq!(result, "Method is POST");
    }

    #[test]
    fn test_multiple_interpolations() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("secret123"));

        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), "456".to_string());

        let ctx = InterpolationContext::new(
            headers,
            path_params,
            HashMap::new(),
            None,
            Method::GET,
        );

        let result = ctx.interpolate(
            "API Key: ${request.headers[\"x-api-key\"]}, ID: ${request.path.id}"
        );
        assert_eq!(result, "API Key: secret123, ID: 456");
    }
}
