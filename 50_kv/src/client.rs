use std::time::Duration;

use anyhow::Result;
use futures::StreamExt;
use kv::{ClientConfig, start_client_with_config};
use kv::{CommandRequest, KvError, ProstClientStream};
use tokio::time;
use tokio_util::compat::Compat;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config: ClientConfig = toml::from_str(include_str!("../fixtures/client.conf"))?;
    let mut ctrl = start_client_with_config(&config).await?;

    let channel = "lobby";
    start_publishing(ctrl.open_stream().await?, channel)?;

    let stream = ctrl.open_stream().await?;

    let mut client = ProstClientStream::new(stream);

    // 生成一个 HSET 命令
    let cmd = CommandRequest::new_hset("table1", "hello", "world".to_string().into());

    // 发送 HSET 命令
    let data = client.execute_unary(&cmd).await?;
    info!("Got response {:?}", data);

    // subscribe
    let cmd = CommandRequest::new_subscribe(channel);
    let mut stream_res = client.execute_streaming(&cmd).await?;
    let id = stream_res.id;

    // unsubscribe
    let _ = start_unsubscribe(ctrl.open_stream().await?, channel, id)?;

    // no method named next
    // you need to import trait
    while let Some(Ok(data)) = stream_res.next().await {
        println!("Got published data: {:?}", data);
    }

    println!("Done!");

    Ok(())
}

fn start_publishing(stream: Compat<yamux::Stream>, name: &str) -> Result<(), KvError> {
    let cmd = CommandRequest::new_publish(name, vec![1.into(), 2.into(), "hello".into()]);
    tokio::spawn(async move {
        time::sleep(Duration::from_millis(1000)).await;
        let mut client = ProstClientStream::new(stream);
        let res = client.execute_unary(&cmd).await.unwrap();
        println!("Finished publishing: {:?}", res);
    });

    Ok(())
}

fn start_unsubscribe(stream: Compat<yamux::Stream>, name: &str, id: u32) -> Result<(), KvError> {
    let cmd = CommandRequest::new_unsubscribe(name, id as _);
    tokio::spawn(async move {
        time::sleep(Duration::from_millis(2000)).await;
        let mut client = ProstClientStream::new(stream);
        let res = client.execute_unary(&cmd).await.unwrap();
        println!("Finished unsubscribing: {:?}", res);
    });

    Ok(())
}
