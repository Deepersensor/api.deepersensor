use tower_http::set_header::SetResponseHeaderLayer;
use http::HeaderValue;
use tower::Layer;
use tower::util::Either;

pub fn security_headers() -> impl Layer<axum::Router> + Clone {
    // Chain a few static security headers (minimal initial set)
    let strict = SetResponseHeaderLayer::if_not_present(http::header::STRICT_TRANSPORT_SECURITY, HeaderValue::from_static("max-age=63072000; includeSubDomains"));
    let cto = SetResponseHeaderLayer::if_not_present(http::header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    let frame = SetResponseHeaderLayer::if_not_present(http::header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    strict.layer(cto.layer(frame))
}