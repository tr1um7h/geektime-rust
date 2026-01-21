use anyhow::Result;
use kv::{ServerConfig, start_server_with_config};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let addr = "127.0.0.1:9527";
    info!("Start listening on {}", addr);

    let config: ServerConfig = toml::from_str(include_str!("../fixtures/server.conf"))?;
    start_server_with_config(&config).await?;

    Ok(())
}
