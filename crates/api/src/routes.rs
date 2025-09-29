use crate::{auth_middleware::{require_auth, AuthUser}, rate_limit::rate_limit, state::AppState, validation};
use axum::middleware;
use axum::response::sse::{Event, Sse};
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use axum::{
    routing::{get, post},
    Router,
};
use ds_auth::{generate_tokens, hash_password, verify_password};
use ds_core::error::{ApiError, ApiResult};
use ds_model::{ChatChunk, ChatMessage, ChatRequest};
use futures_util::Stream;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
// use std::pin::Pin;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/readiness", get(readiness))
        .route("/metrics", get(metrics))
        .route("/v1/models", get(list_models))
        .route("/v1/auth/signup", post(signup))
        .route("/v1/auth/login", post(login));

    // Protected routes (require JWT authentication)
    let protected_routes = Router::new()
        .route("/v1/chat", post(chat))
        .route("/v1/chat/stream", post(chat_stream_sse))
        .route_layer(middleware::from_fn(require_auth));

    // Merge public and protected routes
    public_routes.merge(protected_routes)
}

// Readiness check for Kubernetes - simpler than health, just checks if server is up
async fn readiness() -> impl IntoResponse {
    (StatusCode::OK, "ready")
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    dependencies: DependencyHealth,
}

#[derive(Serialize)]
struct DependencyHealth {
    database: ServiceStatus,
    ollama: ServiceStatus,
}

#[derive(Serialize)]
struct ServiceStatus {
    healthy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let start = std::time::Instant::now();

    // Check database connectivity
    let db_status = match sqlx::query("SELECT 1 as health_check")
        .fetch_one(&state.db)
        .await
    {
        Ok(_) => ServiceStatus {
            healthy: true,
            error: None,
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
        Err(e) => {
            tracing::error!(error = %e, "database health check failed");
            ServiceStatus {
                healthy: false,
                error: Some(e.to_string()),
                latency_ms: None,
            }
        }
    };

    // Check Ollama connectivity
    let ollama_start = std::time::Instant::now();
    let ollama_status = match state.provider.list_models().await {
        Ok(_) => ServiceStatus {
            healthy: true,
            error: None,
            latency_ms: Some(ollama_start.elapsed().as_millis() as u64),
        },
        Err(e) => {
            tracing::warn!(error = %e, "ollama health check failed");
            ServiceStatus {
                healthy: false,
                error: Some(e.to_string()),
                latency_ms: None,
            }
        }
    };

    let overall_healthy = db_status.healthy && ollama_status.healthy;
    let status_code = if overall_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = HealthResponse {
        status: if overall_healthy {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        dependencies: DependencyHealth {
            database: db_status,
            ollama: ollama_status,
        },
    };

    (status_code, Json(response))
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let mut output = String::from("# HELP deepersensor_info API version information\n");
    output.push_str("# TYPE deepersensor_info gauge\n");
    output.push_str(&format!(
        "deepersensor_info{{version=\"{}\"}} 1\n",
        env!("CARGO_PKG_VERSION")
    ));

    output.push_str("\n# HELP deepersensor_db_pool_size Database connection pool size\n");
    output.push_str("# TYPE deepersensor_db_pool_size gauge\n");
    output.push_str(&format!(
        "deepersensor_db_pool_size{{}} {}\n",
        state.db.size()
    ));

    output.push_str("\n# HELP deepersensor_db_pool_idle Idle database connections\n");
    output.push_str("# TYPE deepersensor_db_pool_idle gauge\n");
    output.push_str(&format!(
        "deepersensor_db_pool_idle{{}} {}\n",
        state.db.num_idle()
    ));

    output.push_str("\n# HELP deepersensor_rate_limit_buckets Active rate limit buckets\n");
    output.push_str("# TYPE deepersensor_rate_limit_buckets gauge\n");
    output.push_str(&format!(
        "deepersensor_rate_limit_buckets{{}} {}\n",
        state.rate_map.len()
    ));

    (StatusCode::OK, output)
}

async fn list_models(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> ApiResult<Json<Vec<String>>> {
    rate_limit(&state, addr.ip()).await?;
    let models = state.provider.list_models().await.map_err(|e| {
        tracing::error!(error = %e, "list models failed");
        ApiError::Internal
    })?;
    Ok(Json(models))
}

#[derive(Deserialize)]
struct ChatIn {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Serialize)]
struct ChatOut {
    model: String,
    content: String,
    done: bool,
}

async fn chat(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(input): Json<ChatIn>,
) -> ApiResult<Json<Vec<ChatOut>>> {
    validate_chat(&input)?;
    
    tracing::info!(
        user_id = %user.user_id,
        model = %input.model,
        message_count = input.messages.len(),
        "chat request"
    );
    
    let stream = state
        .provider
        .chat_stream(ChatRequest {
            model: input.model.clone(),
            messages: input.messages.clone(),
        })
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %user.user_id,
                model = %input.model,
                "chat start failed"
            );
            ApiError::Internal
        })?;
    let mut out = Vec::new();
    futures_util::pin_mut!(stream);
    while let Some(chunk) = stream.next().await {
        let c: ChatChunk = chunk.map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %user.user_id,
                "chat chunk error"
            );
            ApiError::Internal
        })?;
        out.push(ChatOut {
            model: c.model,
            content: c.content,
            done: c.done,
        });
    }
    Ok(Json(out))
}

async fn chat_stream_sse(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(input): Json<ChatIn>,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, axum::Error>>>> {
    validate_chat(&input)?;
    
    tracing::info!(
        user_id = %user.user_id,
        model = %input.model,
        message_count = input.messages.len(),
        "chat stream request"
    );
    
    let stream = state
        .provider
        .chat_stream(ChatRequest {
            model: input.model.clone(),
            messages: input.messages.clone(),
        })
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %user.user_id,
                model = %input.model,
                "chat start failed"
            );
            ApiError::Internal
        })?;
    let mapped = stream.map(|chunk| match chunk {
        Ok(chat_chunk) => {
            let json = serde_json::to_string(&chat_chunk).unwrap_or_else(|_| "{}".to_string());
            Ok(Event::default().event("chunk").data(json))
        }
        Err(e) => {
            let json = serde_json::json!({"error": e.to_string()}).to_string();
            Ok(Event::default().event("error").data(json))
        }
    });
    Ok(Sse::new(mapped))
}

#[derive(Deserialize)]
struct SignupIn {
    email: String,
    password: String,
}
#[derive(Serialize)]
struct SignupOut {
    id: String,
    email: String,
}
#[derive(Deserialize)]
struct LoginIn {
    email: String,
    password: String,
}
#[derive(Serialize)]
struct LoginOut {
    access_token: String,
}

async fn signup(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(input): Json<SignupIn>,
) -> ApiResult<Json<SignupOut>> {
    // Validate email and password using validation helpers
    validation::validate_email(&input.email)?;
    validation::validate_password(&input.password)?;

    // Basic per-IP rate limit reuse (same as list_models/chat) to slow signup abuse
    rate_limit(&state, addr.ip()).await?;

    let hash = hash_password(&input.password).map_err(|e| {
        tracing::error!(error = %e, "password hashing failed");
        ApiError::Internal
    })?;

    let id = Uuid::new_v4();

    match sqlx::query("INSERT INTO users (id,email,password_hash) VALUES ($1,$2,$3)")
        .bind(id)
        .bind(&input.email)
        .bind(&hash)
        .execute(&state.db)
        .await
    {
        Ok(_) => {
            tracing::info!(user_id = %id, email = %input.email, "audit.signup.success");
            Ok(Json(SignupOut {
                id: id.to_string(),
                email: input.email,
            }))
        }
        Err(e) => {
            // Check for unique constraint violation (duplicate email)
            if let Some(db_err) = e.as_database_error() {
                if db_err.is_unique_violation() {
                    tracing::debug!(email = %input.email, "duplicate email signup attempt");
                    return Err(ApiError::Unprocessable("email already registered".into()));
                }
            }

            tracing::error!(error = %e, email = %input.email, "audit.signup.fail");
            Err(ApiError::Internal)
        }
    }
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(input): Json<LoginIn>,
) -> ApiResult<Json<LoginOut>> {
    // Apply rate limiting to slow brute force attempts
    rate_limit(&state, addr.ip()).await?;

    let rec_opt = sqlx::query("SELECT id, email, password_hash FROM users WHERE email=$1")
        .bind(&input.email)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "login query failed");
            ApiError::Internal
        })?;

    let rec = rec_opt.ok_or_else(|| {
        tracing::debug!(email = %input.email, ip = %addr.ip(), "login attempt for non-existent user");
        ApiError::Unauthorized
    })?;

    use sqlx::Row;
    let id: uuid::Uuid = rec.try_get("id").map_err(|_| ApiError::Internal)?;
    let _email: String = rec.try_get("email").map_err(|_| ApiError::Internal)?;
    let password_hash: String = rec
        .try_get("password_hash")
        .map_err(|_| ApiError::Internal)?;

    let (valid, needs_rehash) = verify_password(&input.password, &password_hash).map_err(|e| {
        tracing::error!(error = %e, "password verification failed");
        ApiError::Internal
    })?;

    if !valid {
        tracing::warn!(user_id = %id, email = %input.email, ip = %addr.ip(), "audit.login.fail.invalid_password");
        return Err(ApiError::Unauthorized);
    }

    // Rehash password if needed (parameters changed)
    if needs_rehash {
        if let Ok(new_hash) = hash_password(&input.password) {
            let _ = sqlx::query("UPDATE users SET password_hash=$1 WHERE id=$2")
                .bind(&new_hash)
                .bind(id)
                .execute(&state.db)
                .await;
            tracing::debug!(user_id = %id, "password rehashed with updated parameters");
        }
    }

    tracing::info!(user_id = %id, email = %input.email, "audit.login.success");

    let cfg = state.config();
    let token = generate_tokens(
        &id.to_string(),
        &cfg.security.jwt_issuer,
        &cfg.security.jwt_secret,
        cfg.access_ttl(),
    )
    .map_err(|e| {
        tracing::error!(error = %e, "token generation failed");
        ApiError::Internal
    })?;

    Ok(Json(LoginOut {
        access_token: token,
    }))
}

fn validate_chat(input: &ChatIn) -> ApiResult<()> {
    validation::validate_model_name(&input.model)?;
    
    if input.messages.is_empty() {
        return Err(ApiError::Unprocessable("messages required".into()));
    }
    if input.messages.len() > 64 {
        return Err(ApiError::Unprocessable("too many messages (max 64)".into()));
    }
    
    for m in &input.messages {
        validation::validate_message_content(&m.content, 8000)?;
    }
    
    Ok(())
}
