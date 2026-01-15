use anyhow::Result;
// use async_prost::AsyncProstStream;
// use futures::prelude::*;
// use kv::{CommandRequest, CommandResponse, MemTable, RocksDb, Service, ServiceInner};
// use tokio::net::{TcpListener, TcpStream};
// use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    //     tracing_subscriber::fmt::init();

    //     let addr = "127.0.0.1:9527";
    //     let listener = TcpListener::bind(addr).await?;
    //     info!("start listening on {}", addr);

    //     let service: Service<RocksDb> = ServiceInner::new(RocksDb::new("/tmp/rocksdb"))
    //         .fn_before_send(|res| match res.message.as_ref() {
    //             "" => res.message = "altered. Original message is empty".into(),
    //             s => res.message = format!("altered: {}", s),
    //         })
    //         .into();

    //     loop {
    //         let (stream, addr) = listener.accept().await?;
    //         info!("client {:?} connected", addr);
    //         let svc = service.clone();

    //         tokio::spawn(async move {
    //             let mut stream =
    //                 AsyncProstStream::<_, CommandRequest, CommandResponse, _>::from(stream).for_async();
    //             while let Some(Ok(msg)) = stream.next().await {
    //                 info!("Got a new command: {:?}", msg);
    //                 // let mut resp = CommandResponse::default();
    //                 // resp.status = 404;
    //                 // resp.message = "Not found".to_string();
    //                 let resp = svc.execute(msg);

    //                 stream.send(resp).await.unwrap();
    //             }
    //             info!("client {:?} disconnected", addr);
    //         });
    //     }

    Ok(())
}
