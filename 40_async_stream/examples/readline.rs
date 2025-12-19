use futures::prelude::*;
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncRead, BufReader, Lines},
};

/// LineStream 内部使用 tokio::io::Lines
// 可以使用 #[pin] 来声明某个字段在使用的时候需要被封装为 Pin
#[pin_project]
struct LineStream<R> {
    #[pin]
    lines: Lines<BufReader<R>>,
}

impl<R: AsyncRead> LineStream<R> {
    /// 从 BufReader 创建一个 LineStream
    pub fn new(reader: BufReader<R>) -> Self {
        Self {
            lines: reader.lines(),
        }
    }
}

/// 为 LineStream 实现 Stream trait
impl<R: AsyncRead> Stream for LineStream<R> {
    type Item = std::io::Result<String>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project()
            .lines
            // 虽然 Lines 结构提供了 next_line()，但并没有实现 Stream
            .poll_next_line(cx) // Pin<&mut Self>
            // Result<Option<T>> -> Option<Result<T>>
            .map(Result::transpose)
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let file = fs::File::open("Cargo.toml").await?;
    let reader = BufReader::new(file);
    let mut st = LineStream::new(reader);
    while let Some(Ok(line)) = st.next().await {
        println!("Got: {}", line);
    }

    Ok(())
}
