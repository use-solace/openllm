use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub models_loaded: usize,
}

pub async fn health_check(State(state): State<super::super::AppState>) -> impl IntoResponse {
    let models = state.models.lock().await;
    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: Utc::now(),
        models_loaded: models.len(),
    };
    
    (StatusCode::OK, Json(response))
}
