use std::net::SocketAddr; 
use axum::{routing::{get, post}, Router};
use axum::{extract::{State, ConnectInfo}, http::StatusCode, response::{IntoResponse, Response}, Json};
use serde::{Deserialize, Serialize};
use futures_util::StreamExt;
use ds_core::error::{ApiError, ApiResult};
use ds_model::{ChatMessage, ChatRequest, ChatChunk, ModelProvider};
use crate::{state::AppState, rate_limit::rate_limit};
use axum::response::sse::{Sse, Event};
use futures_util::Stream;
use ds_auth::{hash_password, verify_password, generate_tokens};
use std::pin::Pin;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/v1/models", get(list_models))
        .route("/v1/chat", post(chat))
        .route("/v1/chat/stream", post(chat_stream_sse))
        .route("/v1/auth/signup", post(signup))
        .route("/v1/auth/login", post(login))
}

async fn health() -> impl IntoResponse { (StatusCode::OK, "ok") }
async fn metrics() -> impl IntoResponse { (StatusCode::OK, "# metrics placeholder\n") }

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
    validate_chat(&input)?;
    let stream = state.provider.chat_stream(ChatRequest { model: input.model.clone(), messages: input.messages.clone() }).await.map_err(|e| { tracing::error!(error = %e, "chat start failed"); ApiError::Internal })?;
    let mut out = Vec::new();
    futures_util::pin_mut!(stream);
    while let Some(chunk) = stream.next().await { let c: ChatChunk = chunk.map_err(|e| { tracing::error!(error = %e, "chat chunk error"); ApiError::Internal })?; out.push(ChatOut { model: c.model, content: c.content, done: c.done }); }
    Ok(Json(out))
}

async fn chat_stream_sse(State(state): State<AppState>, ConnectInfo(addr): ConnectInfo<SocketAddr>, Json(input): Json<ChatIn>) -> ApiResult<Sse<impl Stream<Item = Result<Event, axum::Error>>>> {
    rate_limit(&state, addr.ip()).await?;
    validate_chat(&input)?;
    let stream = state.provider.chat_stream(ChatRequest { model: input.model.clone(), messages: input.messages.clone() }).await.map_err(|e| { tracing::error!(error = %e, "chat start failed"); ApiError::Internal })?;
    let mapped = stream.map(|chunk| {
        match chunk {
            Ok(chat_chunk) => {
                let json = serde_json::to_string(&chat_chunk).unwrap_or_else(|_| "{}".to_string());
                Ok(Event::default().event("chunk").data(json))
            }
            Err(e) => {
                let json = serde_json::json!({"error": e.to_string()}).to_string();
                Ok(Event::default().event("error").data(json))
            }
        }
    });
    Ok(Sse::new(mapped))
}

#[derive(Deserialize)]
struct SignupIn { email: String, password: String }
#[derive(Serialize)]
struct SignupOut { id: String, email: String }
#[derive(Deserialize)]
struct LoginIn { email: String, password: String }
#[derive(Serialize)]
struct LoginOut { access_token: String }

async fn signup(State(state): State<AppState>, Json(input): Json<SignupIn>) -> ApiResult<Json<SignupOut>> {
    if input.email.len() > 190 || !input.email.contains('@') { return Err(ApiError::Unprocessable("invalid email".into())); }
    if input.password.len() < 8 { return Err(ApiError::Unprocessable("password too short".into())); }
    let hash = hash_password(&input.password).map_err(|_| ApiError::Internal)?;
    let id = Uuid::new_v4();
    let rec = sqlx::query!("INSERT INTO users (id,email,password_hash) VALUES ($1,$2,$3) RETURNING id, email", id, input.email, hash)
        .fetch_one(&state.db).await.map_err(|e| { tracing::error!(error=%e, "signup insert failed"); ApiError::Internal })?;
    Ok(Json(SignupOut { id: rec.id.to_string(), email: rec.email }))
}

async fn login(State(state): State<AppState>, Json(input): Json<LoginIn>) -> ApiResult<Json<LoginOut>> {
    let rec = sqlx::query!("SELECT id, password_hash FROM users WHERE email=$1", input.email)
        .fetch_optional(&state.db).await.map_err(|e| { tracing::error!(error=%e, "login query failed"); ApiError::Internal })?;
    let rec = rec.ok_or(ApiError::Unauthorized)?;
    let valid = verify_password(&input.password, &rec.password_hash).map_err(|_| ApiError::Internal)?;
    if !valid { return Err(ApiError::Unauthorized); }
    let cfg = state.config();
    let token = generate_tokens(&rec.id.to_string(), &cfg.security.jwt_issuer, &cfg.security.jwt_secret, cfg.access_ttl())
        .map_err(|_| ApiError::Internal)?;
    Ok(Json(LoginOut { access_token: token }))
}

fn validate_chat(input: &ChatIn) -> ApiResult<()> {
    if input.model.trim().is_empty() { return Err(ApiError::Unprocessable("model required".into())); }
    if input.messages.is_empty() { return Err(ApiError::Unprocessable("messages required".into())); }
    if input.messages.len() > 64 { return Err(ApiError::Unprocessable("too many messages".into())); }
    for m in &input.messages { if m.content.len() > 8000 { return Err(ApiError::Unprocessable("message too long".into())); } }
    Ok(())
}
