use crate::Value;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum KvError {
    #[error("Not found for table: {0}, key: {1}")]
    NotFound(String, String),

    #[error("Cannot parse command: {0}")]
    InvalidCommand(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
