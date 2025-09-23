use std::{net::SocketAddr, sync::Arc, time::Duration};
use axum::Router;
use tower::{limit::ConcurrencyLimitLayer, ServiceBuilder, Layer};
use tower_http::{trace::TraceLayer, request_id::{MakeRequestId, PropagateRequestIdLayer, SetRequestIdLayer}};
use ds_core::config::AppConfig;
use ds_model::{ModelProvider, OllamaProvider};
use http::header::HeaderName;
use crate::{state::AppState, cors::build_cors, routes, observability::REQUEST_ID_HEADER};
use uuid::Uuid;

struct MakeRequestUuid;
impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _request: &http::Request<B>) -> Option<http::HeaderValue> {
        let id = Uuid::new_v4().to_string();
        http::HeaderValue::from_str(&id).ok()
    }
}

pub fn build_app(cfg: Arc<AppConfig>) -> AppStateAndRouter {
    let provider = Arc::new(OllamaProvider::new(cfg.ollama.base_url.clone(), Duration::from_millis(cfg.ollama.default_timeout_ms))) as Arc<dyn ModelProvider>;
    let state = AppState::new(provider, cfg.clone());
    let cors = build_cors(&cfg);
    let request_id_header: HeaderName = REQUEST_ID_HEADER.parse().expect("valid x-request-id header name");

    let trace = TraceLayer::new_for_http();

    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(request_id_header.clone(), MakeRequestUuid))
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(trace)
        .layer(ConcurrencyLimitLayer::new(1024));

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
