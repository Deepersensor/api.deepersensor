use axum::{
    extract::Request,
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
/// 
/// This middleware extracts and verifies the JWT token from the Authorization header.
/// The AppState is accessed via request extensions since middleware runs after state is attached.
pub async fn require_auth(
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

    // Get state from request extensions (added by Axum's with_state)
    let state = req
        .extensions()
        .get::<crate::state::AppState>()
        .ok_or_else(|| {
            tracing::error!("app state not found in request extensions");
            ApiError::Internal
        })?;

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
