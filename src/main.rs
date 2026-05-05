mod cache;
mod error;
mod mojang;
mod render;
mod routes;
mod shutdown;
mod types;

use std::net::SocketAddr;
use std::sync::Arc;

use cache::AppCache;
use mojang::MojangClient;
use routes::create_router;
use shutdown::shutdown_signal;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub mojang: MojangClient,
    pub cache: Arc<AppCache>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let mojang = match MojangClient::new() {
        Ok(client) => client,
        Err(err) => {
            tracing::error!(error = %err, "failed to initialize Mojang HTTP client");
            return;
        }
    };

    let state = AppState {
        mojang,
        cache: Arc::new(AppCache::new()),
    };

    let app = create_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    info!(%addr, "starting skinhelm server");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!(error = %err, "failed to bind server address");
            return;
        }
    };

    if let Err(err) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        tracing::error!(error = %err, "server stopped with error");
    }
}
