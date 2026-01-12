use axum::{
    extract::State,
    http::{header, StatusCode},
    response::sse::{Event, KeepAlive},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use async_stream::stream;

use super::super::{AppState, InferenceBackend};

#[derive(Debug, Deserialize)]
pub struct InferenceRequest {
    pub model_id: String,
    pub prompt: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub temperature: Option<f32>,
}

fn default_max_tokens() -> u32 {
    512
}

#[derive(Serialize)]
pub struct InferenceResponse {
    pub model_id: String,
    pub text: String,
    pub tokens_generated: u32,
    pub finish_reason: String,
}

#[derive(Serialize)]
pub struct StreamToken {
    pub token: String,
    pub token_id: u32,
    pub complete: bool,
}

#[derive(Serialize, Deserialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize, Deserialize, Default)]
struct OllamaOptions {
    num_predict: u32,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    done: bool,
}

#[derive(Serialize, Deserialize)]
struct OpenAIChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize)]
struct OpenAIChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Serialize, Deserialize)]
struct OpenAIChoice {
    index: u32,
    message: ChatMessage,
    finish_reason: String,
}

#[derive(Serialize, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct HuggingFaceRequest {
    inputs: String,
    parameters: HuggingFaceParameters,
}

#[derive(Serialize, Deserialize, Default)]
struct HuggingFaceParameters {
    max_new_tokens: u32,
    temperature: f32,
    return_full_text: bool,
}

const OLLAMA_DEFAULT_URL: &str = "http://localhost:11434";
const LLAMA_CPP_DEFAULT_URL: &str = "http://localhost:8080";
const HUGGINGFACE_DEFAULT_URL: &str = "https://api-inference.huggingface.co";
const OPENAI_DEFAULT_URL: &str = "https://api.openai.com/v1";

fn get_backend_url(backend: &InferenceBackend) -> String {
    match backend {
        InferenceBackend::Ollama => std::env::var("OLLAMA_URL").unwrap_or_else(|_| OLLAMA_DEFAULT_URL.to_string()),
        InferenceBackend::Llama => std::env::var("LLAMA_CPP_URL").unwrap_or_else(|_| LLAMA_CPP_DEFAULT_URL.to_string()),
        InferenceBackend::HuggingFace => std::env::var("HUGGINGFACE_URL").unwrap_or_else(|_| HUGGINGFACE_DEFAULT_URL.to_string()),
        InferenceBackend::OpenAI => std::env::var("OPENAI_URL").unwrap_or_else(|_| OPENAI_DEFAULT_URL.to_string()),
    }
}

pub async fn inference_complete(
    State(state): State<AppState>,
    Json(req): Json<InferenceRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let models = state.models.lock().await;

    let model_entry = models
        .iter()
        .find(|m| m.registry_entry.id == req.model_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Model '{}' not found or not loaded. Please register and load it first.", req.model_id),
            )
        })?;

    if !model_entry.registry_entry.loaded {
        return Err((
            StatusCode::PRECONDITION_FAILED,
            format!("Model '{}' is not loaded. Load it first.", req.model_id),
        ));
    }

    let backend_url = get_backend_url(&model_entry.registry_entry.inference);
    let model_id = model_entry.registry_entry.id.clone();
    let inference_backend = model_entry.registry_entry.inference.clone();
    let temperature = req.temperature.unwrap_or(0.7);

    drop(models);

    let result = match inference_backend {
        InferenceBackend::Ollama => ollama_generate(&backend_url, &model_id, &req.prompt, req.max_tokens, temperature).await,
        InferenceBackend::Llama => llama_cpp_completion(&backend_url, &model_id, &req.prompt, req.max_tokens, temperature).await,
        InferenceBackend::HuggingFace => huggingface_inference(&backend_url, &model_id, &req.prompt, req.max_tokens, temperature).await,
        InferenceBackend::OpenAI => openai_chat_completion(&backend_url, &model_id, &req.prompt, req.max_tokens, temperature).await,
    };

    let (text, tokens) = result.map_err(|e| (StatusCode::BAD_GATEWAY, e))?;

    let response = InferenceResponse {
        model_id: req.model_id,
        text,
        tokens_generated: tokens,
        finish_reason: "stop".to_string(),
    };

    Ok((StatusCode::OK, Json(response)))
}

async fn ollama_generate(
    base_url: &str,
    model: &str,
    prompt: &str,
    max_tokens: u32,
    temperature: f32,
) -> Result<(String, u32), String> {
    let client = reqwest::Client::new();

    let request_body = OllamaGenerateRequest {
        model: model.to_string(),
        prompt: prompt.to_string(),
        stream: false,
        options: OllamaOptions {
            num_predict: max_tokens,
            temperature,
        },
    };

    let response = client
        .post(&format!("{}/api/generate", base_url))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Ollama API error: {}", response.status()));
    }

    let ollama_resp: OllamaGenerateResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    let tokens = ollama_resp.response.split_whitespace().count() as u32;
    Ok((ollama_resp.response, tokens))
}

async fn llama_cpp_completion(
    base_url: &str,
    _model: &str,
    prompt: &str,
    max_tokens: u32,
    temperature: f32,
) -> Result<(String, u32), String> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "prompt": prompt,
        "n_predict": max_tokens,
        "temperature": temperature,
        "stream": false
    });

    let response = client
        .post(&format!("{}/v1/completions", base_url))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("llama.cpp request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("llama.cpp API error: {}", response.status()));
    }

    let resp_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse llama.cpp response: {}", e))?;

    let text = resp_json["choices"][0]["text"]
        .as_str()
        .ok_or("Invalid llama.cpp response format")?
        .to_string();

    let tokens = text.split_whitespace().count() as u32;
    Ok((text, tokens))
}

async fn huggingface_inference(
    base_url: &str,
    model: &str,
    prompt: &str,
    max_tokens: u32,
    temperature: f32,
) -> Result<(String, u32), String> {
    let client = reqwest::Client::new();

    let hf_token = std::env::var("HUGGINGFACE_TOKEN")
        .map_err(|_| "HUGGINGFACE_TOKEN not set. Set HF_TOKEN environment variable.")?;

    let request_body = HuggingFaceRequest {
        inputs: prompt.to_string(),
        parameters: HuggingFaceParameters {
            max_new_tokens: max_tokens,
            temperature,
            return_full_text: false,
        },
    };

    let response = client
        .post(&format!("{}/models/{}", base_url, model))
        .header("Authorization", format!("Bearer {}", hf_token))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("HuggingFace request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("HuggingFace API error: {} - {}", status, error_text));
    }

    let resp_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse HuggingFace response: {}", e))?;

    let text = resp_json[0]["generated_text"]
        .as_str()
        .or(resp_json[0].as_str())
        .ok_or("Invalid HuggingFace response format")?
        .to_string();

    let tokens = text.split_whitespace().count() as u32;
    Ok((text, tokens))
}

async fn openai_chat_completion(
    base_url: &str,
    model: &str,
    prompt: &str,
    max_tokens: u32,
    temperature: f32,
) -> Result<(String, u32), String> {
    let client = reqwest::Client::new();

    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| "OPENAI_API_KEY not set. Set OPENAI_API_KEY environment variable.")?;

    let request_body = OpenAIChatCompletionRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        max_tokens,
        temperature,
        stream: false,
    };

    let response = client
        .post(&format!("{}/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("OpenAI request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI API error: {} - {}", status, error_text));
    }

    let openai_resp: OpenAIChatCompletionResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

    let text = openai_resp.choices[0].message.content.clone();
    let tokens = openai_resp.usage.completion_tokens;
    Ok((text, tokens))
}

pub async fn inference_stream(
    State(state): State<AppState>,
    Json(req): Json<InferenceRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let models = state.models.lock().await;

    let model_entry = models
        .iter()
        .find(|m| m.registry_entry.id == req.model_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Model '{}' not found or not loaded. Please register and load it first.", req.model_id),
            )
        })?;

    if !model_entry.registry_entry.loaded {
        return Err((
            StatusCode::PRECONDITION_FAILED,
            format!("Model '{}' is not loaded. Load it first.", req.model_id),
        ));
    }

    let backend_url = get_backend_url(&model_entry.registry_entry.inference);
    let model_id = model_entry.registry_entry.id.clone();
    let inference_backend = model_entry.registry_entry.inference.clone();
    let temperature = req.temperature.unwrap_or(0.7);
    let prompt = req.prompt.clone();

    drop(models);

    let stream: Pin<Box<dyn Stream<Item = Result<Event, std::io::Error>> + Send>> = match inference_backend {
        InferenceBackend::Ollama => Box::pin(ollama_stream_events(backend_url.clone(), model_id.clone(), prompt, req.max_tokens, temperature)),
        InferenceBackend::Llama => Box::pin(llama_cpp_stream_events(backend_url.clone(), model_id.clone(), prompt, req.max_tokens, temperature)),
        InferenceBackend::OpenAI => Box::pin(openai_stream_events(backend_url.clone(), model_id.clone(), prompt, req.max_tokens, temperature)),
        InferenceBackend::HuggingFace => {
            return Err((
                StatusCode::NOT_IMPLEMENTED,
                "Streaming not yet supported for HuggingFace backend".to_string(),
            ));
        }
    };

    let response = (
        [(header::CONTENT_TYPE, "text/event-stream"),
         (header::CACHE_CONTROL, "no-cache"),
         (header::CONNECTION, "keep-alive")],
        axum::response::Sse::new(stream)
            .keep_alive(KeepAlive::default()),
    );

    Ok(response)
}

fn ollama_stream_events(
    base_url: String,
    model: String,
    prompt: String,
    max_tokens: u32,
    temperature: f32,
) -> impl Stream<Item = Result<Event, std::io::Error>> {
    stream! {
        let client = reqwest::Client::new();

        let request_body = OllamaGenerateRequest {
            model: model.clone(),
            prompt: prompt.clone(),
            stream: true,
            options: OllamaOptions {
                num_predict: max_tokens,
                temperature,
            },
        };

        let response = match client
            .post(&format!("{}/api/generate", base_url))
            .json(&request_body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                yield Err(std::io::Error::other(format!("Ollama stream failed: {}", e)));
                return;
            }
        };

        if !response.status().is_success() {
            yield Err(std::io::Error::other(format!("Ollama API error: {}", response.status())));
            return;
        }

        let mut byte_stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let mut token_id = 0u32;

        while let Some(chunk) = byte_stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    yield Err(std::io::Error::other(format!("Ollama read error: {}", e)));
                    return;
                }
            };

            buffer.extend_from_slice(&chunk);

            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let line = String::from_utf8_lossy(&buffer[..pos]).to_string();
                buffer.drain(..=pos);

                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(ollama_resp) = serde_json::from_str::<OllamaGenerateResponse>(&line) {
                    let stream_token = StreamToken {
                        token: ollama_resp.response.clone(),
                        token_id,
                        complete: ollama_resp.done,
                    };
                    token_id += 1;

                    if let Ok(json_data) = serde_json::to_string(&stream_token) {
                        yield Ok(Event::default().event("token").data(json_data));
                    }

                    if ollama_resp.done {
                        return;
                    }
                }
            }
        }
    }
}

fn llama_cpp_stream_events(
    base_url: String,
    _model: String,
    prompt: String,
    max_tokens: u32,
    temperature: f32,
) -> impl Stream<Item = Result<Event, std::io::Error>> {
    stream! {
        let client = reqwest::Client::new();

        let request_body = serde_json::json!({
            "prompt": prompt,
            "n_predict": max_tokens,
            "temperature": temperature,
            "stream": true
        });

        let response = match client
            .post(&format!("{}/v1/completions", base_url))
            .json(&request_body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                yield Err(std::io::Error::other(format!("llama.cpp stream failed: {}", e)));
                return;
            }
        };

        if !response.status().is_success() {
            yield Err(std::io::Error::other(format!("llama.cpp API error: {}", response.status())));
            return;
        }

        let mut byte_stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let mut token_id = 0u32;

        while let Some(chunk) = byte_stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    yield Err(std::io::Error::other(format!("llama.cpp read error: {}", e)));
                    return;
                }
            };

            buffer.extend_from_slice(&chunk);

            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let line = String::from_utf8_lossy(&buffer[..pos]).to_string();
                buffer.drain(..=pos);

                if line.trim().is_empty() || !line.starts_with("data: ") {
                    continue;
                }

                let data = &line[6..];
                if data == "[DONE]" {
                    return;
                }

                if let Ok(resp_json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choices) = resp_json["choices"].as_array() {
                        if let Some(choice) = choices.first() {
                            let text = choice["text"].as_str().unwrap_or("");
                            let finish = choice["finish_reason"].is_null() == false;

                            let stream_token = StreamToken {
                                token: text.to_string(),
                                token_id,
                                complete: finish,
                            };
                            token_id += 1;

                            if let Ok(json_data) = serde_json::to_string(&stream_token) {
                                yield Ok(Event::default().event("token").data(json_data));
                            }

                            if finish {
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn openai_stream_events(
    base_url: String,
    model: String,
    prompt: String,
    max_tokens: u32,
    temperature: f32,
) -> impl Stream<Item = Result<Event, std::io::Error>> {
    stream! {
        let client = reqwest::Client::new();

        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

        let request_body = OpenAIChatCompletionRequest {
            model: model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.clone(),
            }],
            max_tokens,
            temperature,
            stream: true,
        };

        let response = match client
            .post(&format!("{}/chat/completions", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request_body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                yield Err(std::io::Error::other(format!("OpenAI stream failed: {}", e)));
                return;
            }
        };

        if !response.status().is_success() {
            yield Err(std::io::Error::other(format!("OpenAI API error: {}", response.status())));
            return;
        }

        let mut byte_stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let mut token_id = 0u32;

        while let Some(chunk) = byte_stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    yield Err(std::io::Error::other(format!("OpenAI read error: {}", e)));
                    return;
                }
            };

            buffer.extend_from_slice(&chunk);

            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let line = String::from_utf8_lossy(&buffer[..pos]).to_string();
                buffer.drain(..=pos);

                if line.trim().is_empty() || !line.starts_with("data: ") {
                    continue;
                }

                let data = &line[6..];
                if data == "[DONE]" {
                    return;
                }

                if let Ok(resp_json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(choices) = resp_json["choices"].as_array() {
                        if let Some(choice) = choices.first() {
                            let delta = &choice["delta"];
                            let text = delta["content"].as_str().unwrap_or("");
                            let finish = choice["finish_reason"].is_null() == false;

                            if text.is_empty() && !finish {
                                continue;
                            }

                            let stream_token = StreamToken {
                                token: text.to_string(),
                                token_id,
                                complete: finish,
                            };
                            token_id += 1;

                            if let Ok(json_data) = serde_json::to_string(&stream_token) {
                                yield Ok(Event::default().event("token").data(json_data));
                            }

                            if finish {
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
}
