#![allow(dead_code)]

use crate::config::Config;
use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

/// Configuration hot reload manager
pub struct ConfigHotReload {
    config_path: PathBuf,
    tx: broadcast::Sender<Arc<Config>>,
}

impl ConfigHotReload {
    /// Create a new hot reload manager
    pub fn new(config_path: PathBuf) -> Self {
        let (tx, _) = broadcast::channel(16);
        Self { config_path, tx }
    }

    /// Get a receiver for configuration updates
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Config>> {
        self.tx.subscribe()
    }

    /// Start watching the configuration file
    pub async fn start_watching(self: Arc<Self>) -> Result<()> {
        let config_path = self.config_path.clone();
        let tx = self.tx.clone();

        // Spawn blocking task for file watching
        tokio::task::spawn_blocking(move || {
            let (watcher_tx, watcher_rx) = std::sync::mpsc::channel();

            let mut watcher: RecommendedWatcher =
                Watcher::new(watcher_tx, notify::Config::default())
                    .expect("Failed to create watcher");

            watcher
                .watch(&config_path, RecursiveMode::NonRecursive)
                .expect("Failed to watch config file");

            info!("Started watching config file: {:?}", config_path);

            loop {
                match watcher_rx.recv() {
                    Ok(Ok(event)) => {
                        if should_reload(&event) {
                            info!("Config file changed, reloading...");

                            // Add a small delay to ensure file write is complete
                            std::thread::sleep(Duration::from_millis(100));

                            match Config::from_yaml_file(config_path.to_str().unwrap()) {
                                Ok(new_config) => {
                                    info!("Successfully reloaded configuration");
                                    let _ = tx.send(Arc::new(new_config));
                                }
                                Err(e) => {
                                    error!("Failed to reload config: {}", e);
                                    warn!("Continuing with previous configuration");
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        error!("Watch error: {}", e);
                    }
                    Err(e) => {
                        error!("Channel error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}

/// Determine if a file event should trigger a reload
fn should_reload(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hot_reload_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        let hot_reload = ConfigHotReload::new(config_path.clone());
        assert_eq!(hot_reload.config_path, config_path);
    }

    #[tokio::test]
    async fn test_subscribe() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        let hot_reload = ConfigHotReload::new(config_path);
        let mut rx = hot_reload.subscribe();

        // Should be able to subscribe
        assert!(rx.try_recv().is_err()); // No messages yet
    }
}
