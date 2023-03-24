
use std::fs::read;
use std::task::Context;

use futures_util::{StreamExt, Future, FutureExt};
use futures_util::future::FusedFuture;
use futures_util::stream::{SplitStream, SplitSink, FusedStream};
use proto::error::{self, ProtocolError, Result};
use proto::message::Message;
use tokio::sync::mpsc::{UnboundedSender, self, UnboundedReceiver};
use tokio::io::{AsyncRead, AsyncWrite};
use std::task::{Poll, ready};
use std::pin::Pin;
use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;
use crate::connection::Connection;
use futures_util::Sink;
use futures_util::Stream;

#[derive(Debug)]
pub struct ClientStream {
    stream: SplitStream<Connection>,
    outgoing: Option<Outgoing>
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
                    return Poll::Ready(None);
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
    tx: UnboundedSender<Message>
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
    sink: SplitSink<Connection, Message>,
    stream: UnboundedReceiver<Message>,
    buffered: Option<Message>
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
                    return Poll::Pending;
                }
            }
        }
    }
}


#[derive(Debug)]
pub struct Client {
    incoming: Option<SplitStream<Connection>>,
    outgoing: Option<Outgoing>,
    sender: Sender,
}

impl Client {
    pub async fn new_tls(sock: TlsStream<TcpStream>) -> error::Result<Client> {
        let (tx_outgoing, rx_outgoing) = mpsc::unbounded_channel();
        let conn = Connection::new_tls_connection(sock, tx_outgoing.clone());
        let (sink, incoming) = conn.split();
        let sender = Sender { tx: tx_outgoing };

        Ok(Client {
            incoming: Some(incoming),
            outgoing: Some(Outgoing {
                sink,
                stream: rx_outgoing,
                buffered: None
            }),
            sender
        })
    }

    pub fn stream(& mut self) -> error::Result<ClientStream> {
        let stream = self.incoming.take().expect("Stream already configured");
        Ok(ClientStream {
            stream,
            outgoing: self.outgoing.take()
        })
    }

    pub async fn pump_send(&mut self) -> Result<(), ProtocolError> {
        if let Some(outgoing) = self.outgoing.as_mut() {
            outgoing.await?;
        }
        Ok(())
    }

    pub fn sender(&self) -> Sender {
        self.sender.clone()
    }
}