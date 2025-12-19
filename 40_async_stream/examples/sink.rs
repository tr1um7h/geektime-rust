use anyhow::Result;
use futures::prelude::*;
use tokio::{fs::File, io::AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<()> {
    // 一般情况下，我们不太需要直接实现 Stream / Sink / AsyncRead / AsyncWrite trait，
    // 如果的确需要，先看看有没有可以使用的辅助函数，
    // 比如通过 poll_fn / unfold 创建 Stream、通过 unfold 创建 Sink。
    let file_sink = writer(File::create("/tmp/hello").await?);
    // pin_mut 可以把变量 pin 住
    futures::pin_mut!(file_sink);
    if file_sink.send("hello\n").await.is_err() {
        println!("Error on send");
    }
    if file_sink.send("world!\n").await.is_err() {
        println!("Error on send");
    }
    Ok(())
}

/// 使用 unfold 生成一个 Sink 数据结构
fn writer<'a>(file: File) -> impl Sink<&'a str> {
    // 通过 unfold 方法，我们不需要撰写 Sink 的几个方法了，而且可以在一个返回 Future 的闭包中来提供处理逻辑
    sink::unfold(file, |mut file, line: &'a str| async move {
        file.write_all(line.as_bytes()).await?;
        eprint!("Received: {}", line);
        Ok::<_, std::io::Error>(file)
    })
}
