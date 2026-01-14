mod frame;

use bytes::{Bytes, BytesMut};
pub use frame::{FrameCoder, read_frame};
use futures::{SinkExt, StreamExt};
use prost::Message;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

use crate::{CommandRequest, CommandResponse, KvError, Service};

pub struct ProstServerStream<S> {
    inner: Framed<S, LengthDelimitedCodec>,
    service: Service,
}

pub struct ProstClientStream<S> {
    inner: Framed<S, LengthDelimitedCodec>,
}

impl<S> ProstServerStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S, service: Service) -> Self {
        Self {
            inner: Framed::new(stream, LengthDelimitedCodec::new()),
            service: service,
        }
    }

    async fn send(&mut self, msg: CommandResponse) -> Result<(), KvError> {
        //TODO: use LengthDelimitedCodec
        // msg -> bytes -> framedCodec
        let buf = Bytes::from(msg.encode_to_vec());
        self.inner.send(buf).await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<CommandRequest, KvError> {
        // frameCodec -> bytes -> msg
        match self.inner.next().await {
            Some(Ok(buf)) => {
                let req = CommandRequest::decode(buf)?;
                return Ok(req);
            }
            Some(Err(e)) => return Err(e.into()),
            None => return Err(KvError::FrameError),
        }
    }

    pub async fn process(mut self) -> Result<(), KvError> {
        while let Ok(cmd) = self.recv().await {
            info!("Got a new command: {:?}", cmd);
            let res = self.service.execute(cmd);
            self.send(res).await?;
        }

        Ok(())
    }
}

impl<S> ProstClientStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: Framed::new(stream, LengthDelimitedCodec::new()),
        }
    }

    async fn send(&mut self, msg: CommandRequest) -> Result<(), KvError> {
        // msg -> bytes -> framedCodec
        let buf = Bytes::from(msg.encode_to_vec());
        self.inner.send(buf).await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<CommandResponse, KvError> {
        // frameCodec -> bytes -> msg
        match self.inner.next().await {
            Some(Ok(buf)) => {
                let resp = CommandResponse::decode(buf)?;
                return Ok(resp);
            }
            Some(Err(e)) => return Err(e.into()),
            None => return Err(KvError::FrameError),
        }
    }

    pub async fn execute(&mut self, cmd: CommandRequest) -> Result<CommandResponse, KvError> {
        self.send(cmd).await?;
        Ok(self.recv().await?)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use bytes::Bytes;
    use std::net::SocketAddr;
    use tokio::net::{TcpListener, TcpStream};

    use crate::{MemTable, ServiceInner, Value, assert_res_ok};

    use super::*;

    #[tokio::test]
    async fn client_server_basic_communication_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;

        let stream = TcpStream::connect(addr).await?;
        let mut client = ProstClientStream::new(stream);

        // 发送 HSET，等待回应

        let cmd = CommandRequest::new_hset("t1", "k1", "v1".into());
        let res = client.execute(cmd).await.unwrap();

        // 第一次 HSET 服务器应该返回 None
        assert_res_ok(res, &[Value::default()], &[]);

        // 再发一个 HSET
        let cmd = CommandRequest::new_hget("t1", "k1");
        let res = client.execute(cmd).await?;

        // 服务器应该返回上一次的结果
        assert_res_ok(res, &["v1".into()], &[]);

        Ok(())
    }

    #[tokio::test]
    async fn client_server_compression_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;

        let stream = TcpStream::connect(addr).await?;
        let mut client = ProstClientStream::new(stream);

        let v: Value = Bytes::from(vec![0u8; 16384]).into();
        let cmd = CommandRequest::new_hset("t2", "k2", v.clone().into());
        let res = client.execute(cmd).await?;

        assert_res_ok(res, &[Value::default()], &[]);

        let cmd = CommandRequest::new_hget("t2", "k2");
        let res = client.execute(cmd).await?;

        assert_res_ok(res, &[v.into()], &[]);

        Ok(())
    }

    async fn start_server() -> Result<SocketAddr> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let service: Service = ServiceInner::new(MemTable::new()).into();
                let server = ProstServerStream::new(stream, service);
                tokio::spawn(server.process());
            }
        });

        Ok(addr)
    }
}
