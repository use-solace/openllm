use axum::{
    extract::State,
    http::{header, StatusCode},
    response::sse::{Event, KeepAlive},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use futures::stream::iter as stream_iter;

use super::super::AppState;

#[derive(Debug, Deserialize)]
pub struct InferenceRequest {
    pub model_id: String,
    pub prompt: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_max_tokens() -> u32 {
    100
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

fn simulate_token_generation(prompt: &str, max_tokens: u32) -> Vec<String> {
    let prompt_words: Vec<&str> = prompt.split_whitespace().collect();
    let base_responses = [
        "Based on the context, ",
        "Looking at the input, ",
        "Given your prompt, ",
        "Analyzing the request, ",
        "Considering what you've asked, ",
    ];
    
    let mut tokens = Vec::new();
    let mut counter = 0;
    
    for i in 0..max_tokens {
        if i == 0 {
            tokens.push(format!("{} ", base_responses[counter % base_responses.len()]));
        } else if counter % 3 == 0 {
            tokens.push(format!("{} ", prompt_words[counter % prompt_words.len()]));
        } else if counter % 5 == 0 {
            tokens.push(format!(" {} ", ["analysis", "response", "result", "output", "completion"][counter % 5]));
        } else {
            tokens.push(format!(" {}", ["continues", "processing", "generating", "reasoning", "thinking"][counter % 5]));
        }
        counter += 1;
        
        if tokens.iter().map(|s| s.len()).sum::<usize>() > 500 {
            break;
        }
    }
    
    tokens
}

pub async fn inference_complete(
    State(state): State<AppState>,
    Json(req): Json<InferenceRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let models = state.models.lock().await;

    let model = models
        .iter()
        .find(|m| m.registry_entry.id == req.model_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Model '{}' not found or not loaded. Please register and load it first.", req.model_id),
            )
        })?;

    if !model.registry_entry.loaded {
        return Err((
            StatusCode::PRECONDITION_FAILED,
            format!("Model '{}' is not loaded. Load it first.", req.model_id),
        ));
    }

    drop(models);

    let tokens = simulate_token_generation(&req.prompt, req.max_tokens);
    let text: String = tokens.join("");

    let response = InferenceResponse {
        model_id: req.model_id,
        text,
        tokens_generated: tokens.len() as u32,
        finish_reason: "stop".to_string(),
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn inference_stream(
    State(state): State<AppState>,
    Json(req): Json<InferenceRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let models = state.models.lock().await;

    let model = models
        .iter()
        .find(|m| m.registry_entry.id == req.model_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Model '{}' not found or not loaded. Please register and load it first.", req.model_id),
            )
        })?;

    if !model.registry_entry.loaded {
        return Err((
            StatusCode::PRECONDITION_FAILED,
            format!("Model '{}' is not loaded. Load it first.", req.model_id),
        ));
    }

    drop(models);
    
    let tokens = simulate_token_generation(&req.prompt, req.max_tokens);
    
    let stream = stream_iter(tokens.into_iter().enumerate().map(|(i, token)| {
        let is_complete = i == 0;
        let stream_token = StreamToken {
            token,
            token_id: i as u32,
            complete: is_complete,
        };
        
        let json_data = serde_json::to_string(&stream_token)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "JSON error"));
        
        json_data.map(|data| {
            Event::default()
                .event("token")
                .data(data)
        })
    }));
    
    let response = (
        [(header::CONTENT_TYPE, "text/event-stream"),
         (header::CACHE_CONTROL, "no-cache"),
         (header::CONNECTION, "keep-alive")],
        axum::response::Sse::new(stream)
            .keep_alive(KeepAlive::default()),
    );
    
    Ok(response)
}
