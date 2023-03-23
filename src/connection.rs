use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::FramedRead;
use futures_util::stream::StreamExt;
use std::pin::Pin;
use pin_project::pin_project;

use proto::line


#[pin_project]
pub struct Connection<T: AsyncRead + AsyncWrite> {
    #[pin]
    stream: FramedRead<T, IrcCodec>,
}

impl<T: AsyncRead + AsyncWrite> Connection<T> {
    pub fn new(stream: T) -> Connection<T> {
        Connection { 
            stream: FramedRead::new(stream, IrcCodec::new()),
        }
    }

    pub async fn run(mut self: Pin<&mut Self>) {
        loop {
           match self.as_mut().project().stream.next().await {
            Some(v) => match v {
                Ok(val) => {
                    println!("{:?}", val);
                }
                Err(e) => {
                    eprintln!("err reading message: {:?}", e);
                }
           }
           None => {}
        }
        }
    }    
}