use thiserror::Error;
use tokio::sync::mpsc;

use crate::{message::Message, response::Response};

pub type Result<T, E = ProtocolError> = ::std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("channel error occurred")]
    SendError(#[source] mpsc::error::SendError<Message>),
    #[error("an io error occurred")]
    Io(#[source] std::io::Error),
    #[error("ping timeout reached")]
    PingTimeout,
    #[error("server error")]
    ServerError,
    #[error("invalid message: {}", string)]
    InvalidMessage {
        string: String,
        #[source]
        cause: MessageParseError,
    },
}

impl From<std::io::Error> for ProtocolError {
    fn from(e: std::io::Error) -> ProtocolError {
        ProtocolError::Io(e)
    }
}

#[derive(Debug, Error)]
pub enum MessageParseError {
    #[error("empty message")]
    EmptyMessage,
    #[error("invalid command")]
    InvalidCommand,
    #[error("invalid amount of arguments")]
    InvalidArgumentCount,
    #[error("no line delimiter")]
    MissingCRLF,
    #[error("command error response")]
    ErrResponse(Response),
    #[error("error decoding line: {}", string)]
    LineError {
        string: String,
        #[source]
        cause: LineCodecError,
    },
}

impl From<Response> for MessageParseError {
    fn from(value: Response) -> Self {
        MessageParseError::ErrResponse(value)
    }
}

#[derive(Debug, Error)]
pub enum LineCodecError {
    #[error("line too loing")]
    MaxLineLengthExceeded,

    #[error("io error")]
    Io(#[source] std::io::Error),

    #[error("encoding error, {0}")]
    InvalidEncoding(String),
}

impl From<std::io::Error> for LineCodecError {
    fn from(e: std::io::Error) -> LineCodecError {
        LineCodecError::Io(e)
    }
}
