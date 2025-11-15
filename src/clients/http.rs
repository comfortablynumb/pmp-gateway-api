use crate::config::HttpClientConfig;
use anyhow::Result;
use reqwest::{Client, Method};
use std::collections::HashMap;
use std::time::Duration;
use tracing::debug;

/// HTTP client with connection pooling
#[derive(Debug, Clone)]
pub struct HttpClient {
    config: HttpClientConfig,
    client: Client,
}

impl HttpClient {
    /// Create a new HTTP client from configuration
    pub fn new(config: HttpClientConfig) -> Result<Self> {
        let client = Client::builder()
            .pool_max_idle_per_host(config.max_connections)
            .timeout(Duration::from_secs(config.timeout))
            .build()?;

        Ok(Self { config, client })
    }

    /// Execute an HTTP request with retry logic
    pub async fn execute_request(
        &self,
        method: &str,
        uri: &str,
        headers: HashMap<String, String>,
        body: Option<String>,
        query_params: HashMap<String, String>,
    ) -> Result<HttpResponse> {
        let url = format!("{}{}", self.config.base_url, uri);
        let method_obj = Method::from_bytes(method.as_bytes())?;

        debug!(
            "Executing HTTP request: {} {} with {} headers, {} query params",
            method,
            url,
            headers.len(),
            query_params.len()
        );

        // Determine retry config
        let max_retries = self
            .config
            .retry
            .as_ref()
            .map(|r| r.max_retries)
            .unwrap_or(0);
        let initial_backoff = self
            .config
            .retry
            .as_ref()
            .map(|r| r.initial_backoff_ms)
            .unwrap_or(100);
        let max_backoff = self
            .config
            .retry
            .as_ref()
            .map(|r| r.max_backoff_ms)
            .unwrap_or(5000);

        let mut attempt = 0;
        let mut last_error: Option<reqwest::Error> = None;

        loop {
            let mut request = self.client.request(method_obj.clone(), &url);

            // Add default headers from client config
            for (key, value) in &self.config.headers {
                request = request.header(key, value);
            }

            // Add request-specific headers (these override defaults)
            for (key, value) in &headers {
                request = request.header(key, value);
            }

            // Add query parameters
            for (key, value) in &query_params {
                request = request.query(&[(key.clone(), value.clone())]);
            }

            // Add body if present
            if let Some(ref body_content) = body {
                request = request.body(body_content.clone());
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let headers = response
                        .headers()
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                        .collect();

                    let body = response.text().await?;

                    debug!(
                        "HTTP response received: status={}, body_len={}",
                        status,
                        body.len()
                    );

                    return Ok(HttpResponse {
                        status,
                        headers,
                        body,
                    });
                }
                Err(e) => {
                    last_error = Some(e);
                    attempt += 1;

                    if attempt > max_retries {
                        break;
                    }

                    // Calculate exponential backoff
                    let backoff = (initial_backoff * 2_u64.pow(attempt - 1)).min(max_backoff);
                    debug!(
                        "Request failed, retrying in {}ms (attempt {})",
                        backoff, attempt
                    );
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                }
            }
        }

        Err(last_error.unwrap().into())
    }

    #[allow(dead_code)]
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }
}

/// HTTP response structure
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpResponse {
    /// Check if the response was successful (2xx status code)
    #[allow(dead_code)]
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_creation() {
        let config = HttpClientConfig {
            base_url: "https://api.example.com".to_string(),
            headers: HashMap::new(),
            min_connections: 1,
            max_connections: 10,
            timeout: 30,
            retry: None,
        };

        let client = HttpClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_http_response_is_success() {
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: "OK".to_string(),
        };
        assert!(response.is_success());

        let error_response = HttpResponse {
            status: 404,
            headers: HashMap::new(),
            body: "Not Found".to_string(),
        };
        assert!(!error_response.is_success());
    }
}
