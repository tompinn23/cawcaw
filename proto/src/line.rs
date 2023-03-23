
use tokio_util::codec::Decoder;
use bytes::{Buf, BufMut, BytesMut, Bytes};

use std::{io, cmp, fmt, fs::read};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct IrcCodec {
    next_index: usize,
    max_length: usize,
}

impl IrcCodec {
    pub fn new() -> IrcCodec {
        IrcCodec { next_index: 0, max_length: 512 }
    }
    pub fn new_break_length(max_length: usize) -> IrcCodec {
        IrcCodec { 
            max_length,
            ..IrcCodec::new()
        }
    }
}

impl Decoder for IrcCodec {
    type Item = BytesMut;
    type Error = IrcCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            let read_to = cmp::min(self.max_length.saturating_add(1), src.len());
            let mut iter = src[self.next_index..read_to].iter().peekable();
            let mut i = 0;
            let newline_offset = loop {
                match (iter.next(), iter.peek()) {
                    (Some(n), Some(p)) => {
                        if *n == b'\r' && **p == b'\n' {
                            break Some(i);
                        }
                    }
                    (Some(_), None) => break None,
                    (None, Some(_)) => break None,
                    (None, None) => break None
                }
                i = i + 1;
            };
            match newline_offset {
                Some(offset) => {
                    let newline_index = offset + self.next_index;
                    self.next_index = 0;
                    let line = src.split_to(newline_index - 1);
                    return Ok(Some(line))
                }
                None if src.len() > self.max_length => {
                    return Err(IrcCodecError::MaxLineLengthExceeded);
                }
                None => {
                    self.next_index = read_to;
                    return Ok(None);
                }
            }
        }
    }
}


#[derive(Debug)]
pub enum IrcCodecError {
    MaxLineLengthExceeded,
    Io(io::Error)
} 


impl fmt::Display for IrcCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrcCodecError::MaxLineLengthExceeded => write!(f, "max line length exceeded"),
            IrcCodecError::Io(e) => write!(f, "{}", e),
        }
    }
}

impl From<io::Error> for IrcCodecError {
    fn from(e: io::Error) -> IrcCodecError {
        IrcCodecError::Io(e)
    }
}

impl std::error::Error for IrcCodecError {}