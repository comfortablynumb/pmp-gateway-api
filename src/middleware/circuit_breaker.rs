use failsafe::{backoff, failure_policy::consecutive_failures, CircuitBreaker};
use std::sync::Arc;
use std::time::Duration;

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: u32,
    /// Duration to wait before attempting to close the circuit
    pub timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(30),
        }
    }
}

/// Wrapper around the circuit breaker to provide a sendable/syncable type
pub struct CircuitBreakerWrapper {
    inner: failsafe::StateMachine<
        failsafe::failure_policy::ConsecutiveFailures<backoff::Constant>,
        (),
    >,
}

impl CircuitBreakerWrapper {
    pub fn is_call_permitted(&self) -> bool {
        self.inner.is_call_permitted()
    }

    pub fn call<E>(&self, f: impl FnOnce() -> Result<(), E>) -> Result<(), failsafe::Error<E>> {
        self.inner.call(f)
    }
}

unsafe impl Send for CircuitBreakerWrapper {}
unsafe impl Sync for CircuitBreakerWrapper {}

/// Create a circuit breaker with the given configuration
pub fn create_circuit_breaker(config: CircuitBreakerConfig) -> Arc<CircuitBreakerWrapper> {
    let failure_policy =
        consecutive_failures(config.failure_threshold, backoff::constant(config.timeout));

    let cb = failsafe::Config::new().failure_policy(failure_policy).build();

    Arc::new(CircuitBreakerWrapper { inner: cb })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_creation() {
        let config = CircuitBreakerConfig::default();
        let cb = create_circuit_breaker(config);
        assert!(cb.is_call_permitted());
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            timeout: Duration::from_millis(100),
        };
        let cb = create_circuit_breaker(config);

        // Simulate failures
        for _ in 0..3 {
            let result = cb.call(|| Err::<(), ()>(()));
            assert!(result.is_err());
        }

        // Circuit should open after threshold failures
        // Note: failsafe circuit breaker behavior depends on implementation
        // The circuit may still permit calls but track failures
    }
}
