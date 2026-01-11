use axum::{routing::{get, post}, Router};
use chrono::{DateTime, Utc};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

mod v1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InferenceBackend {
    #[serde(rename = "ollama")]
    Ollama,
    #[serde(rename = "llama")]
    Llama,
    #[serde(rename = "huggingface")]
    HuggingFace,
    #[serde(rename = "openai")]
    OpenAI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelCapability {
    #[serde(rename = "chat")]
    Chat,
    #[serde(rename = "vision")]
    Vision,
    #[serde(rename = "embedding")]
    Embedding,
    #[serde(rename = "completion")]
    Completion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LatencyProfile {
    #[serde(rename = "extreme")]
    Extreme,
    #[serde(rename = "fast")]
    Fast,
    #[serde(rename = "slow")]
    Slow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistryEntry {
    pub id: String,
    pub name: String,
    pub inference: InferenceBackend,
    pub context: u32,
    #[serde(default)]
    pub quant: Option<String>,
    pub capabilities: Vec<ModelCapability>,
    #[serde(default)]
    pub latency: Option<LatencyProfile>,
    pub size_bytes: u64,
    pub loaded: bool,
    pub loaded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct LoadedModel {
    pub registry_entry: ModelRegistryEntry,
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

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, ValueEnum)]
enum LogLevel {
    Info,
    Debug,
    Trace,
}

#[derive(Parser, Debug)]
#[command(name = "openllm-server")]
#[command(author = "Solace Contributors")]
#[command(version = "1.0.0")]
#[command(about = "OpenLLM inference engine - optimizes interactions with Ollama, HuggingFace, llama.cpp, and OpenAI-compatible APIs", long_about = None)]
struct Args {
    #[arg(short, long, default_value = "8080")]
    #[arg(help = "Port to run the server on")]
    port: u16,

    #[arg(short, long, value_enum)]
    #[arg(help = "Log level (info, debug, trace)")]
    log: Option<LogLevel>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let log_level = match args.log {
        Some(LogLevel::Debug) => "debug",
        Some(LogLevel::Trace) => "trace",
        None | Some(LogLevel::Info) => "info",
    };

    tracing_subscriber::fmt::init();

    tracing::info!("OpenLLM Inference Engine v1.0.0");
    tracing::info!("Optimized for Ollama, HuggingFace, llama.cpp, and OpenAI-compatible APIs");

    let state = AppState::default();

    let app = Router::new()
        .route("/health", get(v1::health_check))
        .route("/v1/models", get(v1::list_models))
        .route("/v1/models/register", post(v1::register_model))
        .route("/v1/models/load", post(v1::load_model))
        .route("/v1/models/unload/:model_id", post(v1::unload_model))
        .route("/v1/inference", post(v1::inference_complete))
        .route("/v1/inference/stream", post(v1::inference_stream))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], args.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect(&format!("Failed to bind to port {}", args.port));

    tracing::info!("Server started on http://{}", addr);
    tracing::info!("Available endpoints:");
    tracing::info!("  - GET  /health                 - Health check");
    tracing::info!("  - GET  /v1/models              - List registered models");
    tracing::info!("  - POST /v1/models/register     - Register a model in the registry");
    tracing::info!("  - POST /v1/models/load         - Load a registered model");
    tracing::info!("  - POST /v1/models/unload/:id   - Unload a model");
    tracing::info!("  - POST /v1/inference           - Non-streaming inference");
    tracing::info!("  - POST /v1/inference/stream    - Streaming inference (SSE)");

    tracing::info!("Running with log level: {}", log_level);

    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}
