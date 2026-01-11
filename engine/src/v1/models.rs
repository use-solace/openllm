use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use super::super::{AppState, ModelInfo, LoadedModel};

#[derive(Serialize)]
pub struct ModelListResponse {
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
pub struct LoadModelRequest {
    pub model_id: String,
    pub model_path: Option<String>,
}

#[derive(Serialize)]
pub struct LoadModelResponse {
    pub success: bool,
    pub model_id: String,
    pub message: String,
}

pub async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
    let models = state.models.lock().await;
    let model_infos: Vec<ModelInfo> = models.iter().map(|m| m.info.clone()).collect();
    
    (StatusCode::OK, Json(ModelListResponse { models: model_infos }))
}

pub async fn load_model(
    State(state): State<AppState>,
    Json(req): Json<LoadModelRequest>,
) -> impl IntoResponse {
    let mut models = state.models.lock().await;
    
    if models.iter().any(|m| m.info.id == req.model_id) {
        return (
            StatusCode::CONFLICT,
            Json(LoadModelResponse {
                success: false,
                model_id: req.model_id,
                message: "Model already loaded".to_string(),
            }),
        );
    }
    
    let model_info = ModelInfo {
        id: req.model_id.clone(),
        name: format!("model-{}", req.model_id),
        size_bytes: 4_000_000_000,
        loaded: true,
        loaded_at: Some(Utc::now()),
    };
    
    models.push(LoadedModel {
        info: model_info,
        last_accessed: SystemTime::now(),
    });
    
    (
        StatusCode::CREATED,
        Json(LoadModelResponse {
            success: true,
            model_id: req.model_id,
            message: "Model loaded successfully".to_string(),
        }),
    )
}
