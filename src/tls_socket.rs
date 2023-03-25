use std::net::SocketAddr;

use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_native_tls::TlsStream;
#[derive(Debug)]
#[pin_project(project = SocketProj)]
pub enum Socket<S> {
    Plain(#[pin] S),
    Tls(#[pin] TlsStream<S>),
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for Socket<S> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.project() {
            SocketProj::Plain(socket) => socket.poll_read(cx, buf),
            SocketProj::Tls(socket) => socket.poll_read(cx, buf),
        }
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for Socket<S> {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.project() {
            SocketProj::Plain(socket) => socket.poll_write(cx, buf),
            SocketProj::Tls(socket) => socket.poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.project() {
            SocketProj::Plain(socket) => socket.poll_flush(cx),
            SocketProj::Tls(socket) => socket.poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.project() {
            SocketProj::Plain(socket) => socket.poll_shutdown(cx),
            SocketProj::Tls(socket) => socket.poll_shutdown(cx),
        }
    }
}
