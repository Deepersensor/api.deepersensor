use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::{get, post}, Json, Router};
use ds_core::config::AppConfig;
use ds_core::error::{ApiError, ApiResult};
use ds_model::{ChatChunk, ChatMessage, ChatRequest, ModelProvider, OllamaProvider};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::signal;
use tower::{limit::ConcurrencyLimitLayer, ServiceBuilder};
use tower_http::{cors::{Any, CorsLayer}, trace::TraceLayer};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct AppState { provider: Arc<dyn ModelProvider> }

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = AppConfig::load()?;

    // Logging init
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = match cfg.logging.log_format.as_str() {
        "json" => fmt::layer().json().with_target(false),
        _ => fmt::layer().with_target(false),
    };
    tracing_subscriber::registry().with(env_filter).with(fmt_layer).init();

    let provider = Arc::new(OllamaProvider::new(
        cfg.ollama.base_url.clone(),
        Duration::from_millis(cfg.ollama.default_timeout_ms),
    ));

    let state = AppState { provider };

    let cors = CorsLayer::new().allow_methods(["GET", "POST", "OPTIONS"]) // refined later
        .allow_headers(Any)
        .allow_origin(Any);

    let middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(ConcurrencyLimitLayer::new(1024));

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/chat", post(chat))
        .with_state(state)
        .layer(middleware)
        .layer(cors);

    let addr: SocketAddr = format!("{}:{}", cfg.app.host, cfg.app.port).parse()?;
    info!(%addr, env = %cfg.app.env, "starting server");

    let server = axum::Server::bind(&addr).serve(app.into_make_service());
    let graceful = server.with_graceful_shutdown(shutdown_signal());
    graceful.await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async { signal::ctrl_c().await.expect("install CTRL+C handler"); };
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("sig term");
        term.recv().await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {}, };
    info!("signal received, shutting down");
}

async fn health() -> impl IntoResponse { (StatusCode::OK, "ok") }

async fn list_models(State(state): State<AppState>) -> ApiResult<Json<Vec<String>>> {
    let models = state.provider.list_models().await.map_err(|e| {
        tracing::error!(error = %e, "list models failed");
        ApiError::Internal
    })?;
    Ok(Json(models))
}

#[derive(Deserialize)]
struct ChatIn { model: String, messages: Vec<ChatMessage> }

#[derive(Serialize)]
struct ChatOut { model: String, content: String, done: bool }

async fn chat(State(state): State<AppState>, Json(input): Json<ChatIn>) -> ApiResult<Json<Vec<ChatOut>>> {
    // For now aggregate stub chunks; later stream
    let stream = state.provider.chat_stream(ChatRequest { model: input.model, messages: input.messages }).await.map_err(|e| {
        tracing::error!(error = %e, "chat start failed"); ApiError::Internal
    })?;
    let mut out = Vec::new();
    futures_util::pin_mut!(stream);
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let c: ChatChunk = chunk.map_err(|e| { tracing::error!(error = %e, "chat chunk error"); ApiError::Internal })?;
        out.push(ChatOut { model: c.model, content: c.content, done: c.done });
    }
    Ok(Json(out))
}
