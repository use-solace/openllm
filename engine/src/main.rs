use axum::{routing::{get, post}, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

mod v1;

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub loaded: bool,
    pub loaded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct LoadedModel {
    pub info: ModelInfo,
    pub last_accessed: SystemTime,
}

#[derive(Clone)]
pub struct AppState {
    pub models: Arc<Mutex<Vec<LoadedModel>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            models: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    
    let state = AppState::default();
    
    let app = Router::new()
        .route("/health", get(v1::health_check))
        .route("/v1/models", get(v1::list_models))
        .route("/v1/models/load", post(v1::load_model))
        .route("/v1/inference", post(v1::inference_complete))
        .route("/v1/inference/stream", post(v1::inference_stream))
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("Failed to bind to port 8080");
    
    tracing::info!("OpenLLM server starting on http://0.0.0.0:8080");
    tracing::info!("Available endpoints:");
    tracing::info!("  - GET  /health          - Health check");
    tracing::info!("  - GET  /v1/models       - List loaded models");
    tracing::info!("  - POST /v1/models/load  - Load a model");
    tracing::info!("  - POST /v1/inference    - Non-streaming inference");
    tracing::info!("  - POST /v1/inference/stream - Streaming inference (SSE)");
    
    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}
