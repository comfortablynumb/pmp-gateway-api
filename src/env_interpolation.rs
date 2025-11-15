use regex::Regex;
use std::env;

/// Interpolate environment variables in a string
/// Supports syntax: ${env:VAR_NAME} or ${env:VAR_NAME:default_value}
pub fn interpolate_env_vars(input: &str) -> String {
    let re = Regex::new(r"\$\{env:([^:}]+)(?::([^}]*))?\}").unwrap();

    re.replace_all(input, |caps: &regex::Captures| {
        let var_name = &caps[1];
        let default_value = caps.get(2).map(|m| m.as_str());

        match env::var(var_name) {
            Ok(value) => value,
            Err(_) => {
                if let Some(default) = default_value {
                    default.to_string()
                } else {
                    // Keep the placeholder if no default and var not found
                    caps[0].to_string()
                }
            }
        }
    })
    .to_string()
}

/// Recursively interpolate environment variables in YAML string
pub fn interpolate_yaml_string(yaml_content: &str) -> String {
    interpolate_env_vars(yaml_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_interpolate_env_var_exists() {
        env::set_var("TEST_VAR", "hello");
        let result = interpolate_env_vars("Value: ${env:TEST_VAR}");
        assert_eq!(result, "Value: hello");
        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_interpolate_env_var_with_default() {
        env::remove_var("MISSING_VAR");
        let result = interpolate_env_vars("Value: ${env:MISSING_VAR:default_value}");
        assert_eq!(result, "Value: default_value");
    }

    #[test]
    fn test_interpolate_env_var_missing_no_default() {
        env::remove_var("MISSING_VAR");
        let result = interpolate_env_vars("Value: ${env:MISSING_VAR}");
        assert_eq!(result, "Value: ${env:MISSING_VAR}");
    }

    #[test]
    fn test_interpolate_multiple_vars() {
        env::set_var("VAR1", "first");
        env::set_var("VAR2", "second");
        let result = interpolate_env_vars("${env:VAR1} and ${env:VAR2}");
        assert_eq!(result, "first and second");
        env::remove_var("VAR1");
        env::remove_var("VAR2");
    }

    #[test]
    fn test_interpolate_connection_string() {
        env::set_var("DB_USER", "admin");
        env::set_var("DB_PASS", "secret");
        env::set_var("DB_HOST", "localhost");
        let result =
            interpolate_env_vars("postgres://${env:DB_USER}:${env:DB_PASS}@${env:DB_HOST}/mydb");
        assert_eq!(result, "postgres://admin:secret@localhost/mydb");
        env::remove_var("DB_USER");
        env::remove_var("DB_PASS");
        env::remove_var("DB_HOST");
    }
}
