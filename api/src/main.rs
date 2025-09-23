use axum::{extract::{State, ConnectInfo}, http::{HeaderValue, Method, StatusCode}, response::IntoResponse, routing::{get, post}, Json, Router};
use ds_core::config::AppConfig;
use ds_core::error::{ApiError, ApiResult};
use ds_model::{ChatChunk, ChatMessage, ChatRequest, ModelProvider, OllamaProvider};
use serde::{Deserialize, Serialize};
use std::{net::{SocketAddr, IpAddr}, sync::Arc, time::{Duration, Instant}};
use tokio::signal;
use tower::{limit::ConcurrencyLimitLayer, ServiceBuilder};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use futures_util::StreamExt;
use tracing::{info};
use tracing_subscriber::{fmt, EnvFilter};
use dashmap::DashMap;

#[derive(Clone)]
struct AppState { provider: Arc<dyn ModelProvider>, rate_map: Arc<DashMap<String, TokenBucket>>, cfg: Arc<AppConfig> }

#[derive(Clone)]
struct TokenBucket { tokens: Arc<tokio::sync::Mutex<(u64, Instant)>>, rate_per_min: u64, burst: u64 }

impl TokenBucket {
    fn new(rate_per_min: u64, burst: u64) -> Self { Self { tokens: Arc::new(tokio::sync::Mutex::new((burst, Instant::now()))), rate_per_min, burst } }
    async fn allow(&self) -> bool {
        let per_sec = self.rate_per_min as f64 / 60.0;
        let mut guard = self.tokens.lock().await;
        let (ref mut available, ref mut last) = *guard;
        let now = Instant::now();
        let elapsed = now.duration_since(*last).as_secs_f64();
        if elapsed > 0.0 { let refill = (per_sec * elapsed) as u64; if refill > 0 { *available = (*available + refill).min(self.burst); *last = now; } }
        if *available > 0 { *available -= 1; true } else { false }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Arc::new(AppConfig::load()?);

    // Logging init
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = match cfg.logging.log_format.as_str() { "json" => fmt::layer().json().with_target(false), _ => fmt::layer().with_target(false) };
    tracing_subscriber::registry().with(env_filter).with(fmt_layer).init();

    let provider = Arc::new(OllamaProvider::new(cfg.ollama.base_url.clone(), Duration::from_millis(cfg.ollama.default_timeout_ms)));

    let rate_map = Arc::new(DashMap::new());

    let cors = build_cors(&cfg);

    let middleware = ServiceBuilder::new().layer(TraceLayer::new_for_http()).layer(ConcurrencyLimitLayer::new(1024));

    let app_state = AppState { provider, rate_map, cfg: cfg.clone() };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/chat", post(chat))
        .with_state(app_state)
        .layer(middleware)
        .layer(cors);

    let addr: SocketAddr = format!("{}:{}", cfg.app.host, cfg.app.port).parse()?;
    info!(%addr, env = %cfg.app.env, "starting server");

    let server = axum::Server::bind(&addr).serve(app.into_make_service_with_connect_info::<SocketAddr>());
    let graceful = server.with_graceful_shutdown(shutdown_signal());
    graceful.await?;
    Ok(())
}

fn build_cors(cfg: &AppConfig) -> CorsLayer {
    let origins: Vec<HeaderValue> = cfg.security.allowed_origins.split(',')
        .filter_map(|o| HeaderValue::from_str(o.trim()).ok()).collect();
    let mut layer = CorsLayer::new()
        .allow_methods(cfg.cors.allow_methods.split(',').filter_map(|m| Method::from_bytes(m.trim().as_bytes()).ok()).collect::<Vec<_>>())
        .allow_headers(cfg.cors.allow_headers.split(',').filter_map(|h| HeaderValue::from_str(h.trim()).ok()).collect::<Vec<_>>())
        .expose_headers(cfg.cors.expose_headers.split(',').filter_map(|h| HeaderValue::from_str(h.trim()).ok()).collect::<Vec<_>>());
    if !origins.is_empty() { layer = layer.allow_origin(origins); }
    if cfg.cors.allow_credentials { layer = layer.allow_credentials(true); }
    layer
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

async fn rate_limit(state: &AppState, ip: IpAddr) -> ApiResult<()> {
    if !state.cfg.rate_limit.enabled { return Ok(()); }
    let key = ip.to_string();
    let entry = state.rate_map.entry(key.clone()).or_insert_with(|| TokenBucket::new(state.cfg.rate_limit.requests_per_minute, state.cfg.rate_limit.burst));
    if !entry.allow().await { return Err(ApiError::RateLimited); }
    Ok(())
}
