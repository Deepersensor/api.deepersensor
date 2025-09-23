use std::{net::SocketAddr, sync::Arc, time::Duration};
use axum::Router;
use tower::{limit::ConcurrencyLimitLayer, ServiceBuilder};
use axum::http;
use tower_http::request_id::{RequestId, MakeRequestId};
use tower_http::{trace::TraceLayer, request_id::{PropagateRequestIdLayer, SetRequestIdLayer}, limit::RequestBodyLimitLayer};
use ds_core::config::AppConfig;
use ds_model::{ModelProvider, OllamaProvider};
use http::header::HeaderName;
use crate::{state::AppState, cors::build_cors, routes, observability::REQUEST_ID_HEADER};
// security headers layer available (currently not applied)
use uuid::Uuid;

#[derive(Clone)]
struct MakeRequestUuid;
impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _request: &http::Request<B>) -> Option<RequestId> {
        let id = Uuid::new_v4().to_string();
        Some(RequestId::new(http::HeaderValue::from_str(&id).expect("valid uuid header value")))
    }
}

pub async fn build_app(cfg: Arc<AppConfig>) -> AppStateAndRouter {
    let provider = Arc::new(OllamaProvider::new(cfg.ollama.base_url.clone(), Duration::from_millis(cfg.ollama.default_timeout_ms))) as Arc<dyn ModelProvider>;
    let db = sqlx::PgPool::connect_lazy(cfg.database_url()).expect("valid db url");
    let state = AppState::new(provider, cfg.clone(), db);
    let cors = build_cors(&cfg);
    let request_id_header: HeaderName = REQUEST_ID_HEADER.parse().expect("valid x-request-id header name");

    let trace = TraceLayer::new_for_http()
        .make_span_with(|req: &http::Request<_>| {
            let method = req.method().clone();
            let uri = req.uri().path().to_string();
            tracing::info_span!("request", %method, %uri, status = tracing::field::Empty)
        })
        .on_response(|res: &http::Response<_>, latency: std::time::Duration, span: &tracing::Span| {
            let status = res.status().as_u16();
            span.record("status", &tracing::field::display(status));
            tracing::info!(parent: span, status, latency_ms = latency.as_millis(), "request.completed");
        });
    let body_limit = RequestBodyLimitLayer::new(cfg.http.max_request_size_bytes as usize);

    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(request_id_header.clone(), MakeRequestUuid))
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(trace)
        .layer(body_limit)
        .layer(ConcurrencyLimitLayer::new(1024));

    // Security headers (applied globally)
    let strict = SetResponseHeaderLayer::if_not_present(
        http::header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"));
    let cto = SetResponseHeaderLayer::if_not_present(
        http::header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"));
    let frame = SetResponseHeaderLayer::if_not_present(
        http::header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"));
    let csp = SetResponseHeaderLayer::if_not_present(
        http::HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'; connect-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; object-src 'none'"));
    let referrer = SetResponseHeaderLayer::if_not_present(
        http::HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"));
    let perms = SetResponseHeaderLayer::if_not_present(
        http::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), microphone=(), camera=(), fullscreen=(self)"));

    let router = Router::new()
        .merge(routes::routes())
        .layer(strict)
        .layer(cto)
        .layer(frame)
        .layer(csp)
        .layer(referrer)
        .layer(perms)
        .layer(middleware)
        .layer(cors);
    AppStateAndRouter { state, router }
}

#[derive(Clone)]
pub struct AppStateAndRouter { pub state: AppState, pub router: Router<AppState> }

pub fn server_addr(cfg: &AppConfig) -> SocketAddr { format!("{}:{}", cfg.app.host, cfg.app.port).parse().expect("invalid bind address") }
