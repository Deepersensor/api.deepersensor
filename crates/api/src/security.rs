use tower_http::set_header::SetResponseHeaderLayer;
use axum::http::{self, HeaderValue};
use tower::{Layer, ServiceBuilder};

#[allow(dead_code)]
pub fn security_headers() -> impl Layer<axum::routing::Route> + Clone {
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

    ServiceBuilder::new()
        .layer(strict)
        .layer(cto)
        .layer(frame)
        .layer(csp)
        .layer(referrer)
        .layer(perms)
}