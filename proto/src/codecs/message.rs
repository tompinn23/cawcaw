use tokio_util::codec::{Decoder, Encoder};

use crate::message::Message;
use memchr::memmem;
use std::str::FromStr;

use super::LineCodec;
use crate::error::{MessageParseError, ProtocolError};

pub struct MessageCodec {
    inner: LineCodec,
}

impl MessageCodec {
    pub fn new(label: &str) -> Result<MessageCodec, MessageParseError> {
        let line = LineCodec::new(label).map_err(|e| MessageParseError::LineError {
            string: "failed to make codec".to_owned(),
            cause: e,
        })?;
        Ok(MessageCodec { inner: line })
    }
}

impl Encoder<Message> for MessageCodec {
    type Error = ProtocolError;

    fn encode(&mut self, msg: Message, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        let mut msg = msg.to_string();
        let crlf = match memmem::find(msg.as_bytes(), b"\r\n") {
            Some(n) => n,
            None => {
                return Err(ProtocolError::InvalidMessage {
                    string: "No CRLF delimeter".to_string(),
                    cause: MessageParseError::MissingCRLF,
                })
            }
        };
        msg.truncate(crlf + 2);
        match self.inner.encode(msg, dst) {
            Ok(_) => Ok(()),
            Err(e) => Err(ProtocolError::InvalidMessage {
                string: "line error".to_string(),
                cause: MessageParseError::LineError {
                    string: format!("error encoding line with: {}", self.inner.name()),
                    cause: e,
                },
            }),
        }
    }
}

impl Decoder for MessageCodec {
    type Item = Message;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.inner
            .decode(src)
            .map_err(|e| ProtocolError::InvalidMessage {
                string: "line error".to_string(),
                cause: MessageParseError::LineError {
                    string: format!("failed to decode line with: {}", self.inner.name()),
                    cause: e,
                },
            })
            .and_then(|res| res.map_or(Ok(None), |msg| msg.parse::<Message>().map(Some)))
    }
}
