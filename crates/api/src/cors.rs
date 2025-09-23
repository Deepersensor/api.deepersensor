use axum::http::{HeaderValue, Method, header::HeaderName};
use tower_http::cors::CorsLayer;
use ds_core::config::AppConfig;
use std::str::FromStr;

pub fn build_cors(cfg: &AppConfig) -> CorsLayer {
    let origins: Vec<HeaderValue> = cfg.security.allowed_origins.split(',')
        .filter_map(|o| HeaderValue::from_str(o.trim()).ok()).collect();
    let allow_headers = cfg.cors.allow_headers.split(',').filter_map(|h| HeaderName::from_str(h.trim()).ok()).collect::<Vec<_>>();
    let expose_headers = cfg.cors.expose_headers.split(',').filter_map(|h| HeaderName::from_str(h.trim()).ok()).collect::<Vec<_>>();

    let mut layer = CorsLayer::new()
        .allow_methods(cfg.cors.allow_methods.split(',').filter_map(|m| Method::from_bytes(m.trim().as_bytes()).ok()).collect::<Vec<_>>())
        .allow_headers(allow_headers)
        .expose_headers(expose_headers);
    if !origins.is_empty() { layer = layer.allow_origin(origins); }
    if cfg.cors.allow_credentials { layer = layer.allow_credentials(true); }
    layer
}
