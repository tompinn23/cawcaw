use std::fs::read;
use std::pin::Pin;

use connection::Connection;
use tokio::{net::TcpListener};
use tokio_native_tls::native_tls::{Identity, self};

mod connection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:6667").await?;
    let der = read("certs.p12").expect("Failed to read certificate");
    let cert = Identity::from_pkcs12(&der, "")?;
    let tls_acceptor = tokio_native_tls::TlsAcceptor::from(native_tls::TlsAcceptor::builder(cert).build()?);
    loop {
        let (socket, _) = listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();
        tokio::spawn(async move {
            let tls_stream = tls_acceptor.accept(socket).await.expect("acccept error");
            let mut client = Connection::new(tls_stream);
            Pin::new(& mut client).run().await;
        });
    }
}