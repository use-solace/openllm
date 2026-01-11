use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use super::super::{
    AppState, LoadedModel, ModelRegistryEntry, InferenceBackend, ModelCapability, LatencyProfile,
};

#[derive(Serialize)]
pub struct ModelListResponse {
    pub models: Vec<ModelRegistryEntry>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterModelRequest {
    pub id: String,
    pub name: String,
    pub inference: InferenceBackend,
    pub context: u32,
    #[serde(default)]
    pub quant: Option<String>,
    pub capabilities: Vec<ModelCapability>,
    #[serde(default)]
    pub latency: Option<LatencyProfile>,
    #[serde(default = "default_size_bytes")]
    pub size_bytes: u64,
}

fn default_size_bytes() -> u64 {
    4_000_000_000
}

#[derive(Serialize)]
pub struct RegisterModelResponse {
    pub success: bool,
    pub model: ModelRegistryEntry,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct LoadModelRequest {
    pub model_id: String,
}

#[derive(Serialize)]
pub struct LoadModelResponse {
    pub success: bool,
    pub model_id: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct UnloadModelResponse {
    pub success: bool,
    pub model_id: String,
    pub message: String,
}

pub async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
    let models = state.models.lock().await;
    let model_entries: Vec<ModelRegistryEntry> = models.iter().map(|m| m.registry_entry.clone()).collect();

    (StatusCode::OK, Json(ModelListResponse { models: model_entries }))
}

pub async fn register_model(
    State(state): State<AppState>,
    Json(req): Json<RegisterModelRequest>,
) -> impl IntoResponse {
    let mut models = state.models.lock().await;

    if models.iter().any(|m| m.registry_entry.id == req.id) {
        return (
            StatusCode::CONFLICT,
            Json(RegisterModelResponse {
                success: false,
                model: ModelRegistryEntry {
                    id: req.id.clone(),
                    name: req.name.clone(),
                    inference: req.inference.clone(),
                    context: req.context,
                    quant: req.quant.clone(),
                    capabilities: req.capabilities.clone(),
                    latency: req.latency.clone(),
                    size_bytes: req.size_bytes,
                    loaded: false,
                    loaded_at: None,
                },
                message: "Model with this ID already registered".to_string(),
            }),
        );
    }

    let registry_entry = ModelRegistryEntry {
        id: req.id.clone(),
        name: req.name.clone(),
        inference: req.inference.clone(),
        context: req.context,
        quant: req.quant.clone(),
        capabilities: req.capabilities.clone(),
        latency: req.latency.clone(),
        size_bytes: req.size_bytes,
        loaded: false,
        loaded_at: None,
    };

    models.push(LoadedModel {
        registry_entry: registry_entry.clone(),
        last_accessed: SystemTime::now(),
    });

    (
        StatusCode::CREATED,
        Json(RegisterModelResponse {
            success: true,
            model: registry_entry,
            message: "Model registered successfully".to_string(),
        }),
    )
}

pub async fn load_model(
    State(state): State<AppState>,
    Json(req): Json<LoadModelRequest>,
) -> impl IntoResponse {
    let mut models = state.models.lock().await;

    if let Some(model) = models.iter_mut().find(|m| m.registry_entry.id == req.model_id) {
        if model.registry_entry.loaded {
            return (
                StatusCode::CONFLICT,
                Json(LoadModelResponse {
                    success: false,
                    model_id: req.model_id,
                    message: "Model already loaded".to_string(),
                }),
            );
        }

        model.registry_entry.loaded = true;
        model.registry_entry.loaded_at = Some(Utc::now());
        model.last_accessed = SystemTime::now();

        return (
            StatusCode::OK,
            Json(LoadModelResponse {
                success: true,
                model_id: req.model_id,
                message: "Model loaded successfully".to_string(),
            }),
        );
    }

    (
        StatusCode::NOT_FOUND,
        Json(LoadModelResponse {
            success: false,
            model_id: req.model_id,
            message: "Model not found in registry".to_string(),
        }),
    )
}

pub async fn unload_model(
    State(state): State<AppState>,
    axum::extract::Path(model_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let mut models = state.models.lock().await;

    if let Some(model) = models.iter_mut().find(|m| m.registry_entry.id == model_id) {
        model.registry_entry.loaded = false;
        model.registry_entry.loaded_at = None;

        return (
            StatusCode::OK,
            Json(UnloadModelResponse {
                success: true,
                model_id,
                message: "Model unloaded successfully".to_string(),
            }),
        );
    }

    (
        StatusCode::NOT_FOUND,
        Json(UnloadModelResponse {
            success: false,
            model_id,
            message: "Model not found in registry".to_string(),
        }),
    )
}
