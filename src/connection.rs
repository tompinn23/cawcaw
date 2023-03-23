use pin_project::pin_project;
use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;

use proto::transport::Transport;

#[pin_project(project = ConnectionProj)]
pub enum Connection {
    Unsecured(#[pin] Transport<TcpStream>),
    Secured(#[pin] Transport<TlsStream<TcpStream>>),
}
