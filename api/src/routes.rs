use std::net::SocketAddr; 
use axum::{routing::{get, post}, Router};
use axum::{extract::{State, ConnectInfo}, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use futures_util::StreamExt;
use ds_core::error::{ApiError, ApiResult};
use ds_model::{ChatMessage, ChatRequest, ChatChunk, ModelProvider};
use crate::{state::AppState, rate_limit::rate_limit};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/chat", post(chat))
}

async fn health() -> impl IntoResponse { (StatusCode::OK, "ok") }

async fn list_models(State(state): State<AppState>, ConnectInfo(addr): ConnectInfo<SocketAddr>) -> ApiResult<Json<Vec<String>>> {
    rate_limit(&state, addr.ip()).await?;
    let models = state.provider.list_models().await.map_err(|e| { tracing::error!(error = %e, "list models failed"); ApiError::Internal })?;
    Ok(Json(models))
}

#[derive(Deserialize)]
struct ChatIn { model: String, messages: Vec<ChatMessage> }

#[derive(Serialize)]
struct ChatOut { model: String, content: String, done: bool }

async fn chat(State(state): State<AppState>, ConnectInfo(addr): ConnectInfo<SocketAddr>, Json(input): Json<ChatIn>) -> ApiResult<Json<Vec<ChatOut>>> {
    rate_limit(&state, addr.ip()).await?;
    let stream = state.provider.chat_stream(ChatRequest { model: input.model, messages: input.messages }).await.map_err(|e| { tracing::error!(error = %e, "chat start failed"); ApiError::Internal })?;
    let mut out = Vec::new();
    futures_util::pin_mut!(stream);
    while let Some(chunk) = stream.next().await { let c: ChatChunk = chunk.map_err(|e| { tracing::error!(error = %e, "chat chunk error"); ApiError::Internal })?; out.push(ChatOut { model: c.model, content: c.content, done: c.done }); }
    Ok(Json(out))
}
