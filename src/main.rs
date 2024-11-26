use axum::routing::{get, post};
use axum::{Extension, Router};
use client_bigtable::{Bigtable, BigtableImpl};
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;
use tokio::net::TcpListener;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "String::default")]
    pub port: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let format = tracing_subscriber::fmt::layer()
        .json()
        .with_file(true)
        .with_level(true)
        .with_target(true);

    let env_filter = EnvFilter::builder()
        .with_env_var("LOG_LEVEL")
        .try_from_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::Registry::default()
        .with(format)
        .with(env_filter);

    let config =
        envy::from_env::<Config>().map_err(|e| anyhow::anyhow!("failed to load config: {}", e))?;

    let bigtable_client = client_bigtable::BigtableImpl::try_new().await?;

    let listener = TcpListener::bind(format!("0.0.0.0:{}", &config.port))
        .await
        .map_err(|e| anyhow::anyhow!("failed to bind port: {}", e))?;

    let router = Router::new()
        .route("/mutate_rows", post(mutate_rows))
        .route("/read_rows", get(read_rows))
        .layer(Extension(bigtable_client));

    axum::serve(listener, router)
        .await
        .map_err(|e| anyhow::anyhow!("failed to run axum router: {}", e))?;

    Ok(())
}

fn mutate_rows(
    Extension(mut bigtable_client): Extension<BigtableImpl>,
) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        match bigtable_client
            .mutate_row(client_bigtable::MutateRowsInput {
                table_name: String::from("metrics"),
                row_key: format!("{}#{}#{}", "device_id", "cpu_usage", ""),
                entries: Vec::new(),
            })
            .await
        {
            Ok(_) => (),
            Err(e) => tracing::error!("failed to mutate rows: {}", e),
        }
    })
}

fn read_rows(
    Extension(mut bigtable_client): Extension<BigtableImpl>,
) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        match bigtable_client
            .read_rows(client_bigtable::ReadRowsInput {
                table_name: String::from("metrics"),
                row_key: String::from("device_id#cpu_usage"),
            })
            .await
        {
            Ok(_) => (),
            Err(e) => tracing::error!("failed to read rows: {}", e),
        }
    })
}
