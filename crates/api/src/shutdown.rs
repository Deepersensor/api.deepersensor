use tracing::info;

pub async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.expect("install CTRL+C handler"); };
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("sig term");
        term.recv().await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {}, };
    info!("signal received, shutting down");
}
