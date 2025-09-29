use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use ds_auth::verify_jwt;
use ds_core::error::ApiError;

/// Extracted user claims from JWT
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: String,
    pub email: Option<String>,
}

/// JWT authentication middleware extractor
pub async fn require_auth(
    State(state): State<crate::state::AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    // Expect "Bearer <token>" format
    if !auth_header.starts_with("Bearer ") {
        tracing::warn!("invalid authorization header format");
        return Err(ApiError::Unauthorized);
    }

    let token = auth_header.strip_prefix("Bearer ").unwrap();
    let cfg = state.config();

    // Verify JWT
    let claims =
        verify_jwt(token, &cfg.security.jwt_secret, &cfg.security.jwt_issuer).map_err(|e| {
            tracing::warn!(error = %e, "jwt verification failed");
            ApiError::Unauthorized
        })?;

    // Extract user info from claims
    let user = AuthUser {
        user_id: claims.sub,
        email: claims.email,
    };

    // Insert user into request extensions for handlers to access
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}
