use std::net::SocketAddr;
use std::sync::Arc;
use std::task::Context;

use crate::server::ServerState;
use crate::tls_socket::Socket;
use futures_util::future::FusedFuture;
use futures_util::stream::{FusedStream, SplitSink, SplitStream};
use futures_util::Sink;
use futures_util::Stream;
use futures_util::{Future, StreamExt};
use proto::codecs::MessageCodec;
use proto::error::{self, ProtocolError, Result};
use proto::message::Message;
use proto::prefix::Prefix;
use proto::transport::Transport;
use std::pin::Pin;
use std::task::{ready, Poll};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_util::codec::Framed;

#[derive(Debug)]
pub struct ClientStream {
    stream: SplitStream<Transport<Socket<TcpStream>>>,
    outgoing: Option<Outgoing>,
}

impl ClientStream {
    pub async fn collect(mut self) -> error::Result<Vec<Message>> {
        let mut output = Vec::new();
        while let Some(msg) = self.next().await {
            match msg {
                Ok(m) => output.push(m),
                Err(e) => return Err(e),
            }
        }

        Ok(output)
    }

}

impl FusedStream for ClientStream {
    fn is_terminated(&self) -> bool {
        false
    }
}

impl Stream for ClientStream {
    type Item = Result<Message, error::ProtocolError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(outgoing) = self.as_mut().outgoing.as_mut() {
            match Pin::new(outgoing).poll(cx) {
                Poll::Ready(Ok(())) => {
                    // assure that we wake up again to check the incoming stream.
                    cx.waker().wake_by_ref();
                    //return Poll::Ready(None);
                }
                Poll::Ready(Err(e)) => {
                    cx.waker().wake_by_ref();
                    return Poll::Ready(Some(Err(e)));
                }
                Poll::Pending => (),
            }
        }

        match ready!(Pin::new(&mut self.as_mut().stream).poll_next(cx)) {
            Some(Ok(msg)) => {
                //self.state.handle_message(&msg)?;
                return Poll::Ready(Some(Ok(msg)));
            }
            other => Poll::Ready(other),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sender {
    tx: UnboundedSender<Message>,
}

impl Sender {
    pub fn send<M: Into<Message>>(&self, msg: M) -> error::Result<()> {
        self.tx
            .send(msg.into())
            .map_err(|e| ProtocolError::SendError(e))
    }
}

#[derive(Debug)]
pub struct Outgoing {
    sink: SplitSink<Transport<Socket<TcpStream>>, Message>,
    stream: UnboundedReceiver<Message>,
    buffered: Option<Message>,
}

impl Outgoing {
    fn try_start_send(
        &mut self,
        cx: &mut Context<'_>,
        msg: Message,
    ) -> Poll<Result<(), ProtocolError>> {
        debug_assert!(self.buffered.is_none());
        match Pin::new(&mut self.sink).poll_ready(cx)? {
            Poll::Ready(()) => Poll::Ready(Pin::new(&mut self.sink).start_send(msg)),
            Poll::Pending => {
                self.buffered = Some(msg);
                Poll::Pending
            }
        }
    }
}

impl FusedFuture for Outgoing {
    fn is_terminated(&self) -> bool {
        false
    }
}

impl Future for Outgoing {
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        if let Some(msg) = this.buffered.take() {
            ready!(this.try_start_send(cx, msg))?
        }

        loop {
            match this.stream.poll_recv(cx) {
                Poll::Ready(Some(message)) => ready!(this.try_start_send(cx, message))?,
                Poll::Ready(None) => {
                    ready!(Pin::new(&mut this.sink).poll_flush(cx))?;
                    return Poll::Ready(Ok(()));
                }
                Poll::Pending => {
                    ready!(Pin::new(&mut this.sink).poll_flush(cx))?;
                    return Poll::Ready(Ok(()));
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientState {
    registered: bool,
    nick: String,
    realname: String,
    hostname: String
}

#[derive(Debug)]
pub struct Client {
    incoming: Option<SplitStream<Transport<Socket<TcpStream>>>>,
    outgoing: Option<Outgoing>,
    sender: Sender,
    addr: SocketAddr,
}

impl Client {
    pub async fn new(sock: Socket<TcpStream>) -> error::Result<Client> {
        let (tx_outgoing, rx_outgoing) = mpsc::unbounded_channel();
        let addr = match &sock {
            Socket::Plain(s) => s.peer_addr(),
            Socket::Tls(t) => t.get_ref().get_ref().get_ref().peer_addr(),
        }.expect("Socket has no peer address");

        let framed = Framed::new(
            sock,
            MessageCodec::new("utf-8").expect("Failed to create message codec"),
        );
        let conn = Transport::new(framed, tx_outgoing.clone());
        let (sink, incoming) = conn.split();
        let sender = Sender { tx: tx_outgoing };

        Ok(Client {
            incoming: Some(incoming),
            outgoing: Some(Outgoing {
                sink,
                stream: rx_outgoing,
                buffered: None,
            }),
            sender,
            addr,
        })
    }

    pub async fn send_to<M: Into<Message>>(& mut self, msg: M) -> Result<(), ProtocolError> {
        self.sender.send(msg)
    }



    pub fn address(&self) -> SocketAddr {
        self.addr.clone()
    }

    pub fn stream(&mut self) -> error::Result<ClientStream> {
        let stream = self.incoming.take().expect("Stream already configured");
        Ok(ClientStream {
            stream,
            outgoing: self.outgoing.take(),
        })
    }

    pub async fn poll_send(&mut self) -> Result<(), ProtocolError> {
        if let Some(outgoing) = self.outgoing.as_mut() {
            outgoing.await.expect("Failed to poll outgoing messages");
        }
        Ok(())
    }

    pub fn sender(&self) -> Sender {
        self.sender.clone()
    }
}

pub enum MessageError {}
