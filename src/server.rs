use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use halolib::config::Config;
use halolib::http::serve;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "holo=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::parse();

    let db = PgPoolOptions::new()
        .max_connections(50)
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect(&config.database_url)
        .await
        .expect("连接失败");

    serve(config, db).await.unwrap();
}
