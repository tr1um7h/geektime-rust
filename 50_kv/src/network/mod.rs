mod frame;
mod tls;

pub use frame::{FrameCoder, read_frame};
pub use tls::{TlsClientConnector, TlsServerAcceptor};

use bytes::Bytes;
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
            inner: Framed::new(
                stream,
                LengthDelimitedCodec::builder()
                    .length_adjustment(2)
                    .new_codec(),
            ),
            service: service,
        }
    }

    async fn send(&mut self, msg: CommandResponse) -> Result<(), KvError> {
        //TODO: use LengthDelimitedCodec
        // msg -> bytes -> framedCodec
        let mut buf: Vec<u8> = vec![0xB, 0xB];
        buf.extend(Bytes::from(msg.encode_to_vec()));
        let buf1 = Bytes::from(buf);
        self.inner.send(buf1).await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<CommandRequest, KvError> {
        // frameCodec -> bytes -> msg
        match self.inner.next().await {
            Some(Ok(buf)) => {
                println!("{:X?}", &buf[..2]);
                let req = CommandRequest::decode(&buf[2..])?;
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
            inner: Framed::new(
                stream,
                LengthDelimitedCodec::builder()
                    .length_adjustment(2)
                    .new_codec(),
            ),
        }
    }

    async fn send(&mut self, msg: CommandRequest) -> Result<(), KvError> {
        // msg -> bytes -> framedCodec
        let mut buf: Vec<u8> = vec![0xC, 0xC];
        buf.extend(Bytes::from(msg.encode_to_vec()));
        let buf1 = Bytes::from(buf);
        self.inner.send(buf1).await?;

        Ok(())
    }

    async fn recv(&mut self) -> Result<CommandResponse, KvError> {
        // frameCodec -> bytes -> msg
        match self.inner.next().await {
            Some(Ok(buf)) => {
                println!("{:X?}", &buf[..2]);
                let resp = CommandResponse::decode(&buf[2..])?;
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

    use std::net::SocketAddr;

    use super::*;
    use anyhow::Result;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    use crate::{MemTable, ServiceInner, Value, assert_res_ok};

    const CA_CERT: &str = include_str!("../../fixtures/ca.cert");
    const CLIENT_CERT: &str = include_str!("../../fixtures/client.cert");
    const CLIENT_KEY: &str = include_str!("../../fixtures/client.key");
    const SERVER_CERT: &str = include_str!("../../fixtures/server.cert");
    const SERVER_KEY: &str = include_str!("../../fixtures/server.key");

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

    #[tokio::test]
    async fn tls_should_work() -> Result<()> {
        let ca = Some(CA_CERT);

        let addr = start_tls_server(None).await?;

        let connector = TlsClientConnector::new("kvserver.acme.inc", None, ca)?;
        let stream = TcpStream::connect(addr).await?;
        let mut stream = connector.connect(stream).await?;
        stream.write_all(b"hello world!").await?;
        let mut buf = [0; 12];
        stream.read_exact(&mut buf).await?;
        assert_eq!(&buf, b"hello world!");

        Ok(())
    }

    #[tokio::test]
    async fn tls_with_client_cert_should_work() -> Result<()> {
        let client_identity = Some((CLIENT_CERT, CLIENT_KEY));
        let ca = Some(CA_CERT);

        let addr = start_tls_server(ca.clone()).await?;

        let connector = TlsClientConnector::new("kvserver.acme.inc", client_identity, ca)?;
        let stream = TcpStream::connect(addr).await?;
        let mut stream = connector.connect(stream).await?;
        stream.write_all(b"hello world!").await?;
        let mut buf = [0; 12];
        stream.read_exact(&mut buf).await?;
        assert_eq!(&buf, b"hello world!");

        Ok(())
    }

    #[tokio::test]
    async fn tls_with_bad_domain_should_not_work() -> Result<()> {
        let addr = start_tls_server(None).await?;

        let connector = TlsClientConnector::new("kvserver1.acme.inc", None, Some(CA_CERT))?;
        let stream = TcpStream::connect(addr).await?;
        let result = connector.connect(stream).await;

        assert!(result.is_err());

        Ok(())
    }

    async fn start_tls_server(ca: Option<&str>) -> Result<SocketAddr> {
        let acceptor = TlsServerAcceptor::new(SERVER_CERT, SERVER_KEY, ca)?;

        let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = echo.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = echo.accept().await.unwrap();
            let mut stream = acceptor.accept(stream).await.unwrap();
            let mut buf = [0; 12];
            stream.read_exact(&mut buf).await.unwrap();
            stream.write_all(&buf).await.unwrap();
        });

        Ok(addr)
    }
}
