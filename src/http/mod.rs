use std::sync::Arc;

use anyhow::Context;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::add_extension::AddExtensionLayer;
use tower_http::trace::TraceLayer;

pub use error::{Error, ResultExt};

use crate::{config::Config, service::api_router};

mod error;

mod extractor;
pub mod types;

pub type Result<T, E = Error> = anyhow::Result<T, E>;

#[derive(Clone)]
pub struct ApiContext {
    pub config: Arc<Config>,
    #[cfg(any(feature = "postgres"))]
    pub db: sqlx::PgPool,
    #[cfg(any(feature = "sqlite"))]
    pub db: sqlx::SqlitePool,
    pub store: cache::LocalCache<String>,
}

/// .
///
/// # Errors
///
pub async fn serve(
    config: Config,
    #[cfg(any(feature = "postgres"))] db: sqlx::PgPool,
    #[cfg(any(feature = "sqlite"))] db: sqlx::SqlitePool,
) -> anyhow::Result<()> {
    let store = cache::LocalCache::new();
    let app = api_router().layer(
        ServiceBuilder::new()
            .layer(AddExtensionLayer::new(ApiContext {
                config: Arc::new(config),
                db,
                store,
            }))
            .layer(TraceLayer::new_for_http()),
    );

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
