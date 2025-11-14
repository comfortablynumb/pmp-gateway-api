pub mod handler;

use axum::{
    routing::any,
    Router,
};
use handler::AppState;
use tracing::debug;

/// Build the router from configuration
pub fn build_router(state: AppState) -> Router {
    let config = state.config.clone();
    let mut router = Router::new();

    // Register each route from configuration
    for route in &config.routes {
        let path = route.path.clone();
        debug!("Registering route: {} {}", route.method, path);

        // For now, we'll use a simple any() matcher and filter by method in the handler
        // A more sophisticated implementation would use proper method routing
        router = router.route(&path, any(handler::handle_route));
    }

    router.with_state(state)
}
