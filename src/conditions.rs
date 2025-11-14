use crate::config::Condition;
use crate::interpolation::InterpolationContext;
use regex::Regex;

/// Evaluate a condition to determine if a subrequest should be executed
pub fn evaluate_condition(condition: &Condition, context: &InterpolationContext) -> bool {
    match condition {
        Condition::Always => true,

        Condition::FieldExists { field } => {
            // Check if field exists in path params or query params
            context.path_params.contains_key(field) || context.query_params.contains_key(field)
        }

        Condition::FieldEquals { field, value } => {
            // Check path params first, then query params
            if let Some(field_value) = context.path_params.get(field) {
                field_value == value
            } else if let Some(field_value) = context.query_params.get(field) {
                field_value == value
            } else {
                false
            }
        }

        Condition::FieldMatches { field, pattern } => {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(field_value) = context.path_params.get(field) {
                    return re.is_match(field_value);
                } else if let Some(field_value) = context.query_params.get(field) {
                    return re.is_match(field_value);
                }
            }
            false
        }

        Condition::HeaderExists { header } => context.headers.contains_key(header),

        Condition::HeaderEquals { header, value } => {
            if let Some(header_value) = context.headers.get(header) {
                if let Ok(header_str) = header_value.to_str() {
                    return header_str == value;
                }
            }
            false
        }

        Condition::QueryExists { param } => context.query_params.contains_key(param),

        Condition::QueryEquals { param, value } => {
            if let Some(param_value) = context.query_params.get(param) {
                param_value == value
            } else {
                false
            }
        }

        Condition::And { conditions } => conditions.iter().all(|c| evaluate_condition(c, context)),

        Condition::Or { conditions } => conditions.iter().any(|c| evaluate_condition(c, context)),

        Condition::Not { condition } => !evaluate_condition(condition, context),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue, Method};
    use std::collections::HashMap;

    #[test]
    fn test_always_condition() {
        let context = create_test_context();
        assert!(evaluate_condition(&Condition::Always, &context));
    }

    #[test]
    fn test_header_exists() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer token"));

        let context =
            InterpolationContext::new(headers, HashMap::new(), HashMap::new(), None, Method::GET);

        let condition = Condition::HeaderExists {
            header: "authorization".to_string(),
        };

        assert!(evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_header_equals() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("secret123"));

        let context =
            InterpolationContext::new(headers, HashMap::new(), HashMap::new(), None, Method::GET);

        let condition = Condition::HeaderEquals {
            header: "x-api-key".to_string(),
            value: "secret123".to_string(),
        };

        assert!(evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_query_exists() {
        let mut query_params = HashMap::new();
        query_params.insert("filter".to_string(), "active".to_string());

        let context = InterpolationContext::new(
            HeaderMap::new(),
            HashMap::new(),
            query_params,
            None,
            Method::GET,
        );

        let condition = Condition::QueryExists {
            param: "filter".to_string(),
        };

        assert!(evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_and_condition() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("token"));

        let mut query_params = HashMap::new();
        query_params.insert("debug".to_string(), "true".to_string());

        let context =
            InterpolationContext::new(headers, HashMap::new(), query_params, None, Method::GET);

        let condition = Condition::And {
            conditions: vec![
                Condition::HeaderExists {
                    header: "authorization".to_string(),
                },
                Condition::QueryExists {
                    param: "debug".to_string(),
                },
            ],
        };

        assert!(evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_or_condition() {
        let context = InterpolationContext::new(
            HeaderMap::new(),
            HashMap::new(),
            HashMap::new(),
            None,
            Method::GET,
        );

        let condition = Condition::Or {
            conditions: vec![
                Condition::HeaderExists {
                    header: "missing".to_string(),
                },
                Condition::Always,
            ],
        };

        assert!(evaluate_condition(&condition, &context));
    }

    #[test]
    fn test_not_condition() {
        let context = InterpolationContext::new(
            HeaderMap::new(),
            HashMap::new(),
            HashMap::new(),
            None,
            Method::GET,
        );

        let condition = Condition::Not {
            condition: Box::new(Condition::HeaderExists {
                header: "missing".to_string(),
            }),
        };

        assert!(evaluate_condition(&condition, &context));
    }

    fn create_test_context() -> InterpolationContext {
        InterpolationContext::new(
            HeaderMap::new(),
            HashMap::new(),
            HashMap::new(),
            None,
            Method::GET,
        )
    }
}
