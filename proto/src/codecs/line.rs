use bytes::{Buf, BufMut, BytesMut};
use encoding::{label::encoding_from_whatwg_label, EncoderTrap, EncodingRef};
use memchr::memchr;
use tokio_util::codec::{Decoder, Encoder};

use crate::error::LineCodecError;
use std::{cmp, io, fmt};

pub struct LineCodec {
    encoding: EncodingRef,
    next_index: usize,
    max_length: usize,
}

impl LineCodec {
    pub fn new(label: &str) -> Result<LineCodec, LineCodecError> {
        encoding_from_whatwg_label(label)
            .map(|enc| LineCodec {
                encoding: enc,
                next_index: 0,
                max_length: 512,
            })
            .ok_or_else(|| LineCodecError::InvalidEncoding(label.to_string()))
    }
    pub fn new_max_length(label: &str, max_length: usize) -> Result<LineCodec, LineCodecError> {
        encoding_from_whatwg_label(label)
            .map(|enc| LineCodec {
                encoding: enc,
                next_index: 0,
                max_length: max_length,
            })
            .ok_or_else(|| LineCodecError::InvalidEncoding(label.to_string()))
    }
    pub fn name(&self) -> &str {
        self.encoding.name()
    }
}

impl Decoder for LineCodec {
    type Item = String;
    type Error = LineCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }
        let read_to = cmp::min(self.max_length.saturating_add(1), src.len());

        let mut len = match memchr(b'\n', &src[self.next_index..read_to]) {
            Some(n) => n,
            None if src.len() > self.max_length => {
                return Err(LineCodecError::MaxLineLengthExceeded);
            }
            None => {
                self.next_index = src.len();
                return Ok(None);
            }
        };
        let mut buf = src.split_to(len);
        src.advance(1);
        loop {
            match buf.last() {
                Some(b'\r') => { 
                    buf.truncate(len - 1);
                    len = len - 1
                }
                None => return Ok(Some(String::new())),
                _ => break
            }
        }
        self.next_index = 0;
        match self
            .encoding
            .decode(&buf.freeze(), encoding::DecoderTrap::Replace)
        {
            Ok(data) => Ok(Some(data)),
            Err(data) => {
                return Err(LineCodecError::Io(
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        &format!("Failed to decode {} as {}.", data, self.encoding.name())[..],
                    )
                    .into(),
                ));
            }
        }
    }
}

impl Encoder<String> for LineCodec {
    type Error = LineCodecError;
    fn encode(&mut self, msg: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let data = match self
            .encoding
            .encode(&msg, EncoderTrap::Replace)
            .map_err(|data| {
                LineCodecError::Io(
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        &format!("Failed to decode {} as {}.", data, self.encoding.name())[..],
                    )
                    .into(),
                )
            }) {
            Ok(data) => {
                if data.len() > self.max_length {
                    return Err(LineCodecError::MaxLineLengthExceeded);
                }
                dst.reserve(self.max_length);
                dst.put_slice(&data);
                return Ok(());
            }
            Err(e) => Err(e),
        };
        return data;
    }
}

impl fmt::Debug for LineCodec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineCodec")
            .field("encoder", &self.encoding.name())
            .field("next_index", &self.next_index)
            .field("max_length", &self.next_index)
            .finish()
    }
} 