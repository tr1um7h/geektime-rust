use anyhow::Result;
use bytes::Bytes;
use futures::prelude::*;
use kv::{CommandRequest, CommandResponse, Service, ServiceInner, SledDb};
use prost::Message;
use tokio::net::TcpListener;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("start listening on {}", addr);

    let service: Service<SledDb> = ServiceInner::new(SledDb::new("/tmp/sleddb"))
        .fn_before_send(|res| match res.message.as_ref() {
            "" => res.message = "altered. Original message is empty".into(),
            s => res.message = format!("altered: {}", s),
        })
        .into();

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("client {:?} connected", addr);
        let svc = service.clone();

        tokio::spawn(async move {
            let mut stream = Framed::new(stream, LengthDelimitedCodec::new());
            while let Some(Ok(buf)) = stream.next().await {
                let cmd = CommandRequest::decode(buf).unwrap();
                info!("Got a new command: {:?}", cmd);
                let resp: CommandResponse = svc.execute(cmd);
                let msg = Bytes::from(resp.encode_to_vec());
                stream.send(msg).await.unwrap();
            }
            info!("client {:?} disconnected", addr);
        });
    }
}
