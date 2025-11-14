pub mod http;

use crate::config::{ClientConfig, Config};
use anyhow::Result;
use std::collections::HashMap;

pub use http::HttpClient;

/// Client manager that holds all configured clients
#[derive(Debug, Clone)]
pub struct ClientManager {
    http_clients: HashMap<String, HttpClient>,
}

impl ClientManager {
    /// Create a new client manager from configuration
    pub fn from_config(config: &Config) -> Result<Self> {
        let mut http_clients = HashMap::new();

        for (client_id, client_config) in &config.clients {
            match client_config {
                ClientConfig::Http(http_config) => {
                    let client = HttpClient::new(http_config.clone())?;
                    http_clients.insert(client_id.clone(), client);
                }
            }
        }

        Ok(Self { http_clients })
    }

    /// Get an HTTP client by ID
    pub fn get_http_client(&self, client_id: &str) -> Option<&HttpClient> {
        self.http_clients.get(client_id)
    }
}
