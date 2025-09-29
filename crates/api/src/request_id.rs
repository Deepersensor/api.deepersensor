use axum::{
    extract::Request,
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Middleware to ensure every request has a unique request ID
/// Either uses existing header or generates a new UUID
pub async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    // Check if request already has an ID (from nginx or client)
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Store in extensions for handlers to access
    req.extensions_mut().insert(RequestId(request_id.clone()));

    // Add to tracing span
    let span = tracing::info_span!("request", request_id = %request_id);
    let _enter = span.enter();

    let mut response = next.run(req).await;

    // Add request ID to response headers
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, header_value);
    }

    response
}

/// Request ID extractor for handlers
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
