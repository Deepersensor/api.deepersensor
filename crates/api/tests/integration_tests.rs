// Integration tests for DeeperSensor API
// These tests require a running PostgreSQL database
// Run with: cargo test --test integration_tests -- --test-threads=1
//
// Setup:
// 1. Export TEST_DATABASE_URL="postgresql://user:pass@localhost/test_db"
// 2. cargo test --test integration_tests -- --test-threads=1

use anyhow::Result;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt; // for `oneshot`

mod helpers {
    use super::*;
    use ds_core::config::AppConfig;
    use std::sync::Arc;

    pub async fn setup_test_app() -> Result<(Arc<AppConfig>, api::state::AppState, axum::Router)> {
        // Load config from environment (requires TEST_DATABASE_URL or DATABASE_URL)
        let mut cfg = AppConfig::load()?;
        
        // Override with test-specific settings
        if let Ok(test_db_url) = std::env::var("TEST_DATABASE_URL") {
            cfg.database.url = test_db_url;
        }
        
        let cfg = Arc::new(cfg);
        let app = api::app::build_app(cfg.clone()).await;
        
        // Run migrations on test database
        sqlx::migrate!("../../../migrations")
            .run(&app.state.db)
            .await?;
        
        Ok((cfg, app.state, app.router))
    }

    pub async fn cleanup_test_db(pool: &sqlx::PgPool) -> Result<()> {
        sqlx::query("TRUNCATE TABLE users CASCADE")
            .execute(pool)
            .await?;
        Ok(())
    }
}

use helpers::*;

use helpers::*;

#[tokio::test]
async fn test_health_endpoint() -> Result<()> {
    let (_cfg, state, router) = setup_test_app().await?;
    
    let response = router
        .with_state(state.clone())
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await?;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    cleanup_test_db(&state.db).await?;
    Ok(())
}

#[tokio::test]
async fn test_readiness_endpoint() -> Result<()> {
    let (_cfg, state, router) = setup_test_app().await?;
    
    let response = router
        .with_state(state.clone())
        .oneshot(
            Request::builder()
                .uri("/readiness")
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await?;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    cleanup_test_db(&state.db).await?;
    Ok(())
}

#[tokio::test]
async fn test_signup_success() -> Result<()> {
    let (_cfg, state, router) = setup_test_app().await?;
    
    let signup_body = json!({
        "email": "test@example.com",
        "password": "password123"
    });
    
    let response = router
        .with_state(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/signup")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(signup_body.to_string()))
                .unwrap()
        )
        .await?;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    cleanup_test_db(&state.db).await?;
    Ok(())
}

#[tokio::test]
async fn test_signup_duplicate_email() -> Result<()> {
    let (_cfg, state, router) = setup_test_app().await?;
    
    let signup_body = json!({
        "email": "duplicate@example.com",
        "password": "password123"
    });
    
    // First signup should succeed
    let response1 = router
        .clone()
        .with_state(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/signup")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(signup_body.to_string()))
                .unwrap()
        )
        .await?;
    
    assert_eq!(response1.status(), StatusCode::OK);
    
    // Second signup with same email should fail
    let response2 = router
        .with_state(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/signup")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(signup_body.to_string()))
                .unwrap()
        )
        .await?;
    
    assert_eq!(response2.status(), StatusCode::UNPROCESSABLE_ENTITY);
    
    cleanup_test_db(&state.db).await?;
    Ok(())
}

#[tokio::test]
async fn test_signup_weak_password() -> Result<()> {
    let (_cfg, state, router) = setup_test_app().await?;
    
    let signup_body = json!({
        "email": "test@example.com",
        "password": "weak"  // Too short, no numbers
    });
    
    let response = router
        .with_state(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/signup")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(signup_body.to_string()))
                .unwrap()
        )
        .await?;
    
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    
    cleanup_test_db(&state.db).await?;
    Ok(())
}

#[tokio::test]
async fn test_login_success() -> Result<()> {
    let (_cfg, state, router) = setup_test_app().await?;
    
    // First, sign up
    let signup_body = json!({
        "email": "login@example.com",
        "password": "password123"
    });
    
    router
        .clone()
        .with_state(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/signup")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(signup_body.to_string()))
                .unwrap()
        )
        .await?;
    
    // Then login
    let login_body = json!({
        "email": "login@example.com",
        "password": "password123"
    });
    
    let response = router
        .with_state(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/login")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(login_body.to_string()))
                .unwrap()
        )
        .await?;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    cleanup_test_db(&state.db).await?;
    Ok(())
}

#[tokio::test]
async fn test_login_wrong_password() -> Result<()> {
    let (_cfg, state, router) = setup_test_app().await?;
    
    // First, sign up
    let signup_body = json!({
        "email": "wrongpass@example.com",
        "password": "password123"
    });
    
    router
        .clone()
        .with_state(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/signup")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(signup_body.to_string()))
                .unwrap()
        )
        .await?;
    
    // Then try to login with wrong password
    let login_body = json!({
        "email": "wrongpass@example.com",
        "password": "wrongpassword123"
    });
    
    let response = router
        .with_state(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/login")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(login_body.to_string()))
                .unwrap()
        )
        .await?;
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    
    cleanup_test_db(&state.db).await?;
    Ok(())
}

#[tokio::test]
async fn test_metrics_endpoint() -> Result<()> {
    let (_cfg, state, router) = setup_test_app().await?;
    
    let response = router
        .with_state(state.clone())
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await?;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Verify it's Prometheus format
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body_str = String::from_utf8(body.to_vec())?;
    
    assert!(body_str.contains("deepersensor_info"));
    assert!(body_str.contains("deepersensor_db_pool_size"));
    
    cleanup_test_db(&state.db).await?;
    Ok(())
}
