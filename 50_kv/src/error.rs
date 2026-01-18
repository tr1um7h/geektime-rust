use std::io;

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum KvError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Parse cert failed: ca: {0}, cert: {1}")]
    CertifcateParseError(&'static str, &'static str),

    #[error("Cannot convert to {1}")]
    ConvertError(String, &'static str),

    #[error("Cannot parse command: {0}")]
    InvalidCommand(String),

    #[error("Cannot process command {0} with table: {1}, key: {2}. Error: {3}")]
    StorageError(&'static str, String, String, String),

    #[error("Failed to encode protobuf message")]
    EncodeError(#[from] prost::EncodeError),

    #[error("Failed to decode protobuf message")]
    DecodeError(#[from] prost::DecodeError),

    #[error("Frame is larger than max size")]
    FrameError,

    #[error("Failed to io")]
    IoError(MyError),

    #[error("Failed to access sled db")]
    SledError(#[from] sled::Error),

    #[error("Failed to tls")]
    TlsError(#[from] rustls::TLSError),

    // #[error("Failed to acess rocks db")]
    // RocksdbError(#[from] rust_rocksdb::Error),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug)]
pub struct MyError(io::Error);

impl PartialEq for MyError {
    fn eq(&self, other: &Self) -> bool {
        self.0.kind() == other.0.kind()
    }
}

impl From<io::Error> for MyError {
    fn from(error: io::Error) -> Self {
        MyError(error)
    }
}

impl From<io::Error> for KvError {
    fn from(error: io::Error) -> Self {
        KvError::IoError(MyError::from(error))
    }
}
