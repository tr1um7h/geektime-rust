use anyhow::Result;
use bytes::Bytes;
use futures::prelude::*;
use kv::CommandResponse;
use prost::Message;
use tokio::net::TcpListener;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("Start listening on {}", addr);
    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client {:?} connected", addr);

        let mut stream = Framed::new(stream, LengthDelimitedCodec::new());
        tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                info!("Got a new command: {:?}", msg);
                let resp = CommandResponse {
                    status: 404,
                    message: "Not found".to_string(),
                    ..Default::default()
                };
                stream
                    .send(Bytes::from(resp.encode_to_vec()))
                    .await
                    .unwrap();
            }
            info!("Client {:?} disconnected", addr);
        });
    }
}
