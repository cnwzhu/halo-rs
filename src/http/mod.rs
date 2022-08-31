use anyhow::Context;
use async_session::MemoryStore;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::add_extension::AddExtensionLayer;

mod error;

mod extractor;

pub use error::{Error, ResultExt};

pub type Result<T, E = Error> = anyhow::Result<T, E>;

use tower_http::trace::TraceLayer;

use crate::{config::Config, service::api_router};

#[derive(Clone)]
pub struct ApiContext {
    config: Arc<Config>,
    db: PgPool,
    store: MemoryStore,
}

/// .
///
/// # Errors
///
/// This function will return an error if .
pub async fn serve(config: Config, db: PgPool) -> anyhow::Result<()> {
    let store = MemoryStore::new();
    let app = api_router().layer(
        ServiceBuilder::new()
            .layer(AddExtensionLayer::new(ApiContext {
                config: Arc::new(config),
                db,
                store,
            }))
            .layer(TraceLayer::new_for_http()),
    );

    // We use 8080 as our default HTTP server port, it's pretty easy to remember.
    //
    // Note that any port below 1024 needs superuser privileges to bind on Linux,
    // so 80 isn't usually used as a default for that reason.
    axum::Server::bind(&"0.0.0.0:8080".parse()?)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("error running HTTP server")
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::debug!("signal received, starting graceful shutdown");
}
