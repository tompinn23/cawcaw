use pin_project::pin_project;
use tokio::{net::TcpStream, sync::{mpsc::UnboundedSender}};
use tokio_util::codec::Framed;
use tokio_native_tls::TlsStream;
use proto::{transport::Transport, codecs::MessageCodec, message::Message, error::{Result, ProtocolError}};
use futures_util::{Stream, Sink};
use std::task::{Context, Poll};
use std::pin::Pin;

#[derive(Debug)]
#[pin_project(project = ConnectionProj)]
pub enum Connection {
    Unsecured(#[pin] Transport<TcpStream>),
    Secured(#[pin] Transport<TlsStream<TcpStream>>),
}

impl Connection {
    pub fn new_tls_connection(sock: TlsStream<TcpStream>, tx: UnboundedSender<Message>) -> Connection {
        let framed = Framed::new(sock, MessageCodec::new("utf-8").expect("Failed to create message codec"));
        Connection::Secured(Transport::new(framed, tx))
    }
    pub fn new_connection(sock: TcpStream, tx: UnboundedSender<Message>) -> Connection {
        let framed = Framed::new(sock, MessageCodec::new("utf-8").expect("Failed to create message codec"));
        Connection::Unsecured(Transport::new(framed, tx))
    }
}

impl Stream for Connection {
    type Item = Result<Message>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        match self.project() {
            ConnectionProj::Unsecured(inner) => inner.poll_next(cx),
            ConnectionProj::Secured(inner) => inner.poll_next(cx),
        }
    }
}

impl Sink<Message> for Connection {
    type Error = ProtocolError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.project() {
            ConnectionProj::Unsecured(inner) => inner.poll_ready(cx),
            ConnectionProj::Secured(inner) => inner.poll_ready(cx),
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        match self.project() {
            ConnectionProj::Unsecured(inner) => inner.start_send(item),
            ConnectionProj::Secured(inner) => inner.start_send(item),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.project() {
            ConnectionProj::Unsecured(inner) => inner.poll_flush(cx),
            ConnectionProj::Secured(inner) => inner.poll_flush(cx),
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.project() {
            ConnectionProj::Unsecured(inner) => inner.poll_close(cx),
            ConnectionProj::Secured(inner) => inner.poll_close(cx),
        }
    }
}
