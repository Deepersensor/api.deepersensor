mod app; mod cors; mod observability; mod shutdown; mod state; mod rate_limit; mod routes;
use std::sync::Arc;
use tracing::info;
use ds_core::config::AppConfig;
use axum::Server;
use crate::app::{build_app, server_addr};
use crate::observability::init_tracing;
use crate::shutdown::shutdown_signal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Arc::new(AppConfig::load()?);
    init_tracing(&cfg);

    let addr = server_addr(&cfg);
    let app_state_and_router = build_app(cfg.clone());
    info!(%addr, env = %cfg.app.env, "starting server");

    let server = Server::bind(&addr).serve(app_state_and_router.router.into_make_service_with_connect_info::<std::net::SocketAddr>());
    server.with_graceful_shutdown(shutdown_signal()).await?;
    Ok(())
}
