use crate::config::ResponseTransform;
use crate::interpolation::InterpolationContext;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Apply response transformation to a JSON value
pub fn apply_transformation(
    value: Value,
    transform: &ResponseTransform,
    context: &InterpolationContext,
) -> Value {
    let mut result = value;

    // Apply filter if specified
    if let Some(filter) = &transform.filter {
        result = apply_filter(&result, filter);
    }

    // Apply field mappings (renaming)
    if !transform.field_mappings.is_empty() {
        result = apply_field_mappings(result, &transform.field_mappings);
    }

    // Apply include/exclude filters
    if !transform.include_fields.is_empty() || !transform.exclude_fields.is_empty() {
        result = filter_fields(result, &transform.include_fields, &transform.exclude_fields);
    }

    // Apply template if specified
    if let Some(template) = &transform.template {
        result = apply_template(template, &result, context);
    }

    result
}

/// Apply a JSONPath-like filter to extract data
fn apply_filter(value: &Value, filter: &str) -> Value {
    // Simple implementation - supports basic path notation like "data.users" or "results[0]"
    let parts: Vec<&str> = filter.split('.').collect();
    let mut current = value.clone();

    for part in parts {
        if let Some(array_index) = parse_array_access(part) {
            let (field, index) = array_index;
            if !field.is_empty() {
                current = current.get(field).cloned().unwrap_or(Value::Null);
            }
            if let Value::Array(arr) = current {
                current = arr.get(index).cloned().unwrap_or(Value::Null);
            } else {
                return Value::Null;
            }
        } else {
            current = current.get(part).cloned().unwrap_or(Value::Null);
        }
    }

    current
}

/// Parse array access notation like "items\[0\]"
fn parse_array_access(part: &str) -> Option<(&str, usize)> {
    if let Some(start) = part.find('[') {
        if let Some(end) = part.find(']') {
            let field = &part[..start];
            let index_str = &part[start + 1..end];
            if let Ok(index) = index_str.parse::<usize>() {
                return Some((field, index));
            }
        }
    }
    None
}

/// Apply field mappings (rename fields)
fn apply_field_mappings(value: Value, mappings: &HashMap<String, String>) -> Value {
    match value {
        Value::Object(map) => {
            let mut new_map = Map::new();

            for (key, val) in map {
                let new_key = mappings.get(&key).cloned().unwrap_or(key);
                new_map.insert(new_key, apply_field_mappings(val, mappings));
            }

            Value::Object(new_map)
        }
        Value::Array(arr) => Value::Array(
            arr.into_iter()
                .map(|v| apply_field_mappings(v, mappings))
                .collect(),
        ),
        _ => value,
    }
}

/// Filter fields based on include/exclude lists
fn filter_fields(value: Value, include: &[String], exclude: &[String]) -> Value {
    match value {
        Value::Object(map) => {
            let mut new_map = Map::new();

            for (key, val) in map {
                let should_include = if !include.is_empty() {
                    include.contains(&key)
                } else {
                    !exclude.contains(&key)
                };

                if should_include {
                    new_map.insert(key, filter_fields(val, include, exclude));
                }
            }

            Value::Object(new_map)
        }
        Value::Array(arr) => Value::Array(
            arr.into_iter()
                .map(|v| filter_fields(v, include, exclude))
                .collect(),
        ),
        _ => value,
    }
}

/// Apply a template with interpolation
fn apply_template(template: &str, data: &Value, context: &InterpolationContext) -> Value {
    // First interpolate request data
    let mut result = context.interpolate(template);

    // Then interpolate response data using ${response.field} notation
    result = interpolate_response_data(&result, data);

    // Try to parse as JSON, fallback to string
    serde_json::from_str(&result).unwrap_or_else(|_| Value::String(result))
}

/// Interpolate response data into template
fn interpolate_response_data(template: &str, data: &Value) -> String {
    let mut result = template.to_string();

    // Find all ${response.xxx} patterns
    let re = regex::Regex::new(r"\$\{response\.([^}]+)\}").unwrap();

    for cap in re.captures_iter(template) {
        let full_match = &cap[0];
        let field_path = &cap[1];

        let value = extract_field_value(data, field_path);
        let value_str = match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => String::new(),
            _ => value.to_string(),
        };

        result = result.replace(full_match, &value_str);
    }

    result
}

/// Extract a field value from JSON data using dot notation
fn extract_field_value(data: &Value, path: &str) -> Value {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = data.clone();

    for part in parts {
        if let Some((field, index)) = parse_array_access(part) {
            if !field.is_empty() {
                current = current.get(field).cloned().unwrap_or(Value::Null);
            }
            if let Value::Array(arr) = current {
                current = arr.get(index).cloned().unwrap_or(Value::Null);
            } else {
                return Value::Null;
            }
        } else {
            current = current.get(part).cloned().unwrap_or(Value::Null);
        }
    }

    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_apply_filter() {
        let data = json!({
            "data": {
                "users": [
                    {"id": 1, "name": "Alice"},
                    {"id": 2, "name": "Bob"}
                ]
            }
        });

        let result = apply_filter(&data, "data.users");
        assert!(result.is_array());
        assert_eq!(result.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_apply_field_mappings() {
        let data = json!({
            "old_name": "value",
            "keep_this": "data"
        });

        let mut mappings = HashMap::new();
        mappings.insert("old_name".to_string(), "new_name".to_string());

        let result = apply_field_mappings(data, &mappings);
        assert!(result.get("new_name").is_some());
        assert!(result.get("old_name").is_none());
        assert!(result.get("keep_this").is_some());
    }

    #[test]
    fn test_filter_fields_include() {
        let data = json!({
            "field1": "value1",
            "field2": "value2",
            "field3": "value3"
        });

        let include = vec!["field1".to_string(), "field3".to_string()];
        let exclude = vec![];

        let result = filter_fields(data, &include, &exclude);
        assert!(result.get("field1").is_some());
        assert!(result.get("field2").is_none());
        assert!(result.get("field3").is_some());
    }

    #[test]
    fn test_filter_fields_exclude() {
        let data = json!({
            "field1": "value1",
            "field2": "value2",
            "field3": "value3"
        });

        let include = vec![];
        let exclude = vec!["field2".to_string()];

        let result = filter_fields(data, &include, &exclude);
        assert!(result.get("field1").is_some());
        assert!(result.get("field2").is_none());
        assert!(result.get("field3").is_some());
    }
}
