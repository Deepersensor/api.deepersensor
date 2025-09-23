use std::{net::SocketAddr, sync::Arc, time::Duration};
use axum::{Router};
use tower::{limit::ConcurrencyLimitLayer, ServiceBuilder};
use tower_http::trace::TraceLayer;
use ds_core::config::AppConfig;
use ds_model::{ModelProvider, OllamaProvider};
use crate::{state::AppState, cors::build_cors, routes};

pub fn build_app(cfg: Arc<AppConfig>) -> AppStateAndRouter {
    let provider = Arc::new(OllamaProvider::new(cfg.ollama.base_url.clone(), Duration::from_millis(cfg.ollama.default_timeout_ms))) as Arc<dyn ModelProvider>;
    let state = AppState::new(provider, cfg.clone());
    let cors = build_cors(&cfg);
    let middleware = ServiceBuilder::new().layer(TraceLayer::new_for_http()).layer(ConcurrencyLimitLayer::new(1024));
    let router = Router::new()
        .merge(routes::routes())
        .with_state(state.clone())
        .layer(middleware)
        .layer(cors);
    AppStateAndRouter { state, router }
}

#[derive(Clone)]
pub struct AppStateAndRouter { pub state: AppState, pub router: Router<AppState> }

pub fn server_addr(cfg: &AppConfig) -> SocketAddr { format!("{}:{}", cfg.app.host, cfg.app.port).parse().expect("invalid bind address") }
