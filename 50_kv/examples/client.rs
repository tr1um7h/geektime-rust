use anyhow::Result;
use bytes::Bytes;
use futures::prelude::*;
use kv::{CommandRequest, CommandResponse};
use prost::Message;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:9527";
    let stream = TcpStream::connect(addr).await?;

    let mut client = Framed::new(stream, LengthDelimitedCodec::new());

    let cmd = CommandRequest::new_hset("table1", "hello", "world".into());
    let buf = Bytes::from(cmd.encode_to_vec());
    client.send(buf).await?;
    if let Some(Ok(data)) = client.next().await {
        let resp = CommandResponse::decode(data)?;
        info!("Got response: {:?}", resp);
    }

    Ok(())
}
