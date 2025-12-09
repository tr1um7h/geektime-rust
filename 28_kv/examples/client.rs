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
    // 连接服务器
    let stream = TcpStream::connect(addr).await?;

    // 使用 AsyncProstStream 来处理 TCP Frame
    let mut client = Framed::new(stream, LengthDelimitedCodec::new());

    // 生成一个 HSET 命令
    let cmd = CommandRequest::new_hset("table1", "hello", "world".to_string().into());

    // 发送 HSET 命令
    let data = Bytes::from(cmd.encode_to_vec());
    client.send(data).await?;
    if let Some(Ok(data)) = client.next().await {
        let resp = CommandResponse::decode(data).unwrap();
        info!("Got response {:?}", resp);
    }

    Ok(())
}
