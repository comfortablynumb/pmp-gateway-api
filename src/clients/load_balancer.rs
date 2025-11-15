use crate::config::LoadBalanceStrategy;
use rand::Rng;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Load balancer for selecting backends
pub struct LoadBalancer {
    backends: Vec<String>,
    strategy: LoadBalanceStrategy,
    round_robin_counter: Arc<AtomicUsize>,
    connection_counts: Vec<Arc<AtomicUsize>>,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(backends: Vec<String>, strategy: LoadBalanceStrategy) -> Self {
        let connection_counts = backends
            .iter()
            .map(|_| Arc::new(AtomicUsize::new(0)))
            .collect();

        Self {
            backends,
            strategy,
            round_robin_counter: Arc::new(AtomicUsize::new(0)),
            connection_counts,
        }
    }

    /// Select a backend URL based on the load balancing strategy
    pub fn select_backend(&self) -> Option<String> {
        if self.backends.is_empty() {
            return None;
        }

        let index = match self.strategy {
            LoadBalanceStrategy::RoundRobin => self.round_robin(),
            LoadBalanceStrategy::Random => self.random(),
            LoadBalanceStrategy::LeastConnections => self.least_connections(),
        };

        self.backends.get(index).cloned()
    }

    /// Round-robin selection
    fn round_robin(&self) -> usize {
        let current = self.round_robin_counter.fetch_add(1, Ordering::Relaxed);
        current % self.backends.len()
    }

    /// Random selection
    fn random(&self) -> usize {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..self.backends.len())
    }

    /// Least connections selection
    fn least_connections(&self) -> usize {
        let mut min_connections = usize::MAX;
        let mut min_index = 0;

        for (i, count) in self.connection_counts.iter().enumerate() {
            let connections = count.load(Ordering::Relaxed);
            if connections < min_connections {
                min_connections = connections;
                min_index = i;
            }
        }

        min_index
    }

    /// Increment connection count for a backend
    pub fn increment_connections(&self, index: usize) {
        if let Some(count) = self.connection_counts.get(index) {
            count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Decrement connection count for a backend
    pub fn decrement_connections(&self, index: usize) {
        if let Some(count) = self.connection_counts.get(index) {
            count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Get the index of a backend URL
    pub fn get_backend_index(&self, url: &str) -> Option<usize> {
        self.backends.iter().position(|b| b == url)
    }
}

impl Clone for LoadBalancer {
    fn clone(&self) -> Self {
        Self {
            backends: self.backends.clone(),
            strategy: self.strategy.clone(),
            round_robin_counter: Arc::clone(&self.round_robin_counter),
            connection_counts: self.connection_counts.iter().map(Arc::clone).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin() {
        let backends = vec![
            "http://backend1.com".to_string(),
            "http://backend2.com".to_string(),
            "http://backend3.com".to_string(),
        ];

        let lb = LoadBalancer::new(backends.clone(), LoadBalanceStrategy::RoundRobin);

        // Test that round-robin cycles through backends
        assert_eq!(lb.select_backend(), Some("http://backend1.com".to_string()));
        assert_eq!(lb.select_backend(), Some("http://backend2.com".to_string()));
        assert_eq!(lb.select_backend(), Some("http://backend3.com".to_string()));
        assert_eq!(lb.select_backend(), Some("http://backend1.com".to_string()));
    }

    #[test]
    fn test_random() {
        let backends = vec![
            "http://backend1.com".to_string(),
            "http://backend2.com".to_string(),
            "http://backend3.com".to_string(),
        ];

        let lb = LoadBalancer::new(backends.clone(), LoadBalanceStrategy::Random);

        // Test that random selection returns one of the backends
        for _ in 0..10 {
            let selected = lb.select_backend().unwrap();
            assert!(backends.contains(&selected));
        }
    }

    #[test]
    fn test_least_connections() {
        let backends = vec![
            "http://backend1.com".to_string(),
            "http://backend2.com".to_string(),
        ];

        let lb = LoadBalancer::new(backends, LoadBalanceStrategy::LeastConnections);

        // Initially, should select backend 0
        assert_eq!(lb.select_backend(), Some("http://backend1.com".to_string()));

        // Simulate connections to backend 0
        lb.increment_connections(0);
        lb.increment_connections(0);

        // Should now select backend 1 (fewer connections)
        assert_eq!(lb.select_backend(), Some("http://backend2.com".to_string()));
    }
}
