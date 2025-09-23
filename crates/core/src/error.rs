use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Not Found")] NotFound,
    #[error("Unauthorized")] Unauthorized,
    #[error("Forbidden")] Forbidden,
    #[error("Bad Request: {0}")] BadRequest(String),
    #[error("Unprocessable: {0}")] Unprocessable(String),
    #[error("Too Many Requests")] RateLimited,
    #[error("Internal Server Error")] Internal,
}

#[derive(Serialize)]
struct ErrorBody<'a> { error: ErrorObj<'a> }
#[derive(Serialize)]
struct ErrorObj<'a> { code: &'a str, message: &'a str }

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            ApiError::Unprocessable(_) => (StatusCode::UNPROCESSABLE_ENTITY, "unprocessable"),
            ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };
        let msg = self.to_string();
        (status, Json(ErrorBody { error: ErrorObj { code, message: &msg } })).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
