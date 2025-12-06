//! HTTP server

use axum::{
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::Orchestrator;
use super::handlers::{self, AppState};

/// HTTP API Server
pub struct Server {
    router: Router,
    addr: SocketAddr,
}

impl Server {
    /// Create a new server
    pub fn new(orchestrator: Orchestrator, addr: SocketAddr) -> Self {
        let state: AppState = Arc::new(orchestrator);

        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let router = Router::new()
            // Health
            .route("/health", get(handlers::health))

            // Templates
            .route("/api/v1/templates", get(handlers::list_templates))
            .route("/api/v1/templates", post(handlers::create_template))
            .route("/api/v1/templates/:name", get(handlers::get_template))
            .route("/api/v1/templates/:name", delete(handlers::delete_template))

            // Pools
            .route("/api/v1/pools", get(handlers::list_pools))
            .route("/api/v1/pools", post(handlers::create_pool))
            .route("/api/v1/pools/:name", get(handlers::get_pool))
            .route("/api/v1/pools/:name", delete(handlers::delete_pool))
            .route("/api/v1/pools/:name/provision", post(handlers::provision_pool))
            .route("/api/v1/pools/:name/prepare", post(handlers::prepare_pool))

            // VMs
            .route("/api/v1/vms", get(handlers::list_vms))
            .route("/api/v1/vms/:name", get(handlers::get_vm))
            .route("/api/v1/vms/:name", delete(handlers::delete_vm))
            .route("/api/v1/vms/:name/resume", post(handlers::resume_vm))
            .route("/api/v1/vms/:name/save", post(handlers::save_vm))
            .route("/api/v1/vms/:name/reset", post(handlers::reset_vm))
            .route("/api/v1/vms/:name/stop", post(handlers::stop_vm))
            .route("/api/v1/vms/:name/prepare", post(handlers::prepare_vm))
            .route("/api/v1/vms/:name/release", post(handlers::release_vm))

            // Acquire (from pool)
            .route("/api/v1/acquire", post(handlers::acquire_vm))

            // Reconcile
            .route("/api/v1/reconcile", post(handlers::reconcile))

            .layer(TraceLayer::new_for_http())
            .layer(cors)
            .with_state(state);

        Self { router, addr }
    }

    /// Run the server
    pub async fn run(self) -> Result<(), std::io::Error> {
        tracing::info!("Starting API server on {}", self.addr);

        let listener = tokio::net::TcpListener::bind(self.addr).await?;
        axum::serve(listener, self.router).await
    }
}
