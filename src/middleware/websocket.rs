use axum::{
    extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message as TungsteniteMessage};
use tracing::{debug, error, info, warn};

/// WebSocket proxy configuration
#[derive(Debug, Clone)]
pub struct WebSocketProxyConfig {
    /// Backend WebSocket URL
    pub backend_url: String,
    /// Connection timeout in seconds
    pub timeout: u64,
    /// Maximum message size in bytes
    pub max_message_size: usize,
}

impl Default for WebSocketProxyConfig {
    fn default() -> Self {
        Self {
            backend_url: String::new(),
            timeout: 30,
            max_message_size: 64 * 1024 * 1024, // 64 MB
        }
    }
}

/// WebSocket proxy handler
pub async fn websocket_proxy_handler(
    ws: WebSocketUpgrade,
    config: Arc<WebSocketProxyConfig>,
) -> Response {
    ws.on_upgrade(move |socket| websocket_proxy(socket, config))
}

/// Proxy WebSocket connection to backend
async fn websocket_proxy(client_socket: WebSocket, config: Arc<WebSocketProxyConfig>) {
    info!("WebSocket connection established, proxying to {}", config.backend_url);

    // Connect to backend WebSocket
    let backend_result = connect_async(&config.backend_url).await;

    let (backend_ws, _) = match backend_result {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to connect to backend WebSocket: {}", e);
            return;
        }
    };

    let (backend_write, backend_read) = backend_ws.split();
    let (client_write, client_read) = client_socket.split();

    let backend_write = Arc::new(Mutex::new(backend_write));
    let client_write = Arc::new(Mutex::new(client_write));

    // Client -> Backend
    let backend_write_clone = Arc::clone(&backend_write);
    let client_to_backend = async move {
        let mut client_read = client_read;
        while let Some(msg) = client_read.next().await {
            match msg {
                Ok(msg) => {
                    let backend_msg = convert_axum_to_tungstenite(msg);
                    if let Some(backend_msg) = backend_msg {
                        debug!("Forwarding message to backend");
                        let mut backend = backend_write_clone.lock().await;
                        if let Err(e) = backend.send(backend_msg).await {
                            error!("Error sending to backend: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!("Error receiving from client: {}", e);
                    break;
                }
            }
        }
        debug!("Client to backend stream closed");
    };

    // Backend -> Client
    let client_write_clone = Arc::clone(&client_write);
    let backend_to_client = async move {
        let mut backend_read = backend_read;
        while let Some(msg) = backend_read.next().await {
            match msg {
                Ok(msg) => {
                    let client_msg = convert_tungstenite_to_axum(msg);
                    if let Some(client_msg) = client_msg {
                        debug!("Forwarding message to client");
                        let mut client = client_write_clone.lock().await;
                        if let Err(e) = client.send(client_msg).await {
                            error!("Error sending to client: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!("Error receiving from backend: {}", e);
                    break;
                }
            }
        }
        debug!("Backend to client stream closed");
    };

    // Run both directions concurrently
    tokio::select! {
        _ = client_to_backend => {
            info!("Client connection closed");
        }
        _ = backend_to_client => {
            info!("Backend connection closed");
        }
    }

    // Close connections
    let _ = backend_write.lock().await.close().await;
    let _ = client_write.lock().await.close().await;

    info!("WebSocket proxy connection terminated");
}

/// Convert Axum WebSocket message to Tungstenite message
fn convert_axum_to_tungstenite(msg: AxumMessage) -> Option<TungsteniteMessage> {
    match msg {
        AxumMessage::Text(text) => Some(TungsteniteMessage::Text(text)),
        AxumMessage::Binary(data) => Some(TungsteniteMessage::Binary(data)),
        AxumMessage::Ping(data) => Some(TungsteniteMessage::Ping(data)),
        AxumMessage::Pong(data) => Some(TungsteniteMessage::Pong(data)),
        AxumMessage::Close(frame) => {
            if let Some(frame) = frame {
                Some(TungsteniteMessage::Close(Some(
                    tokio_tungstenite::tungstenite::protocol::CloseFrame {
                        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::from(frame.code),
                        reason: frame.reason,
                    },
                )))
            } else {
                Some(TungsteniteMessage::Close(None))
            }
        }
    }
}

/// Convert Tungstenite message to Axum WebSocket message
fn convert_tungstenite_to_axum(msg: TungsteniteMessage) -> Option<AxumMessage> {
    match msg {
        TungsteniteMessage::Text(text) => Some(AxumMessage::Text(text)),
        TungsteniteMessage::Binary(data) => Some(AxumMessage::Binary(data)),
        TungsteniteMessage::Ping(data) => Some(AxumMessage::Ping(data)),
        TungsteniteMessage::Pong(data) => Some(AxumMessage::Pong(data)),
        TungsteniteMessage::Close(frame) => {
            if let Some(frame) = frame {
                Some(AxumMessage::Close(Some(axum::extract::ws::CloseFrame {
                    code: frame.code.into(),
                    reason: frame.reason,
                })))
            } else {
                Some(AxumMessage::Close(None))
            }
        }
        TungsteniteMessage::Frame(_) => None, // Raw frames are not supported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketProxyConfig::default();
        assert_eq!(config.timeout, 30);
        assert_eq!(config.max_message_size, 64 * 1024 * 1024);
    }

    #[test]
    fn test_convert_axum_text_message() {
        let axum_msg = AxumMessage::Text("hello".to_string());
        let tungstenite_msg = convert_axum_to_tungstenite(axum_msg);
        assert!(matches!(tungstenite_msg, Some(TungsteniteMessage::Text(_))));
    }

    #[test]
    fn test_convert_axum_binary_message() {
        let axum_msg = AxumMessage::Binary(vec![1, 2, 3]);
        let tungstenite_msg = convert_axum_to_tungstenite(axum_msg);
        assert!(matches!(tungstenite_msg, Some(TungsteniteMessage::Binary(_))));
    }

    #[test]
    fn test_convert_tungstenite_text_message() {
        let tungstenite_msg = TungsteniteMessage::Text("hello".to_string());
        let axum_msg = convert_tungstenite_to_axum(tungstenite_msg);
        assert!(matches!(axum_msg, Some(AxumMessage::Text(_))));
    }

    #[test]
    fn test_convert_tungstenite_binary_message() {
        let tungstenite_msg = TungsteniteMessage::Binary(vec![1, 2, 3]);
        let axum_msg = convert_tungstenite_to_axum(tungstenite_msg);
        assert!(matches!(axum_msg, Some(AxumMessage::Binary(_))));
    }
}
