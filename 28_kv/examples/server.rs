use anyhow::Result;
use bytes::Bytes;
use futures::prelude::*;
use kv::{CommandRequest, MemTable, Service};
use prost::Message;
use tokio::net::TcpListener;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let service: Service = Service::new(MemTable::new());
    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("Start listening on {}", addr);
    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client {:?} connected", addr);
        let svc = service.clone();

        let mut stream = Framed::new(stream, LengthDelimitedCodec::new());
        tokio::spawn(async move {
            while let Some(Ok(data)) = stream.next().await {
                let cmd = CommandRequest::decode(data).unwrap();
                info!("Got a new command: {:?}", cmd);
                let res = svc.execute(cmd);
                // prost Message trait
                stream.send(Bytes::from(res.encode_to_vec())).await.unwrap();
            }

            info!("Client {:?} disconnected", addr);
        });
    }
}
