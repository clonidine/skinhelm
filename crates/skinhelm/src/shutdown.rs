use tracing::info;

pub async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => info!("shutdown signal received"),
        Err(err) => tracing::error!(error = %err, "failed to listen for shutdown signal"),
    }
}
