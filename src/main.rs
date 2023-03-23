use std::{fs::read, pin};

use futures_util::SinkExt;
use futures_util::StreamExt;
use proto::{codecs::MessageCodec, command::Command, message::Message, transport::Transport};
use tokio::{net::TcpListener, sync::mpsc};
use tokio_native_tls::native_tls::{self, Identity};
use tokio_util::codec::Framed;

mod client;
mod connection;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:6667").await?;
    let der = read("cert.pem").expect("Failed to read certificate");
    let key = read("key.pem").expect("Failed to read key");
    let cert = Identity::from_pkcs8(&der, &key)?;
    let tls_acceptor =
        tokio_native_tls::TlsAcceptor::from(native_tls::TlsAcceptor::builder(cert).build()?);
    loop {
        let (socket, _) = listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();
        tokio::spawn(async move {
            let stream = tls_acceptor
                .accept(socket)
                .await
                .expect("Failed to accept tls connection");
            let framing = Framed::new(
                stream,
                MessageCodec::new("utf-8").expect("Failed to construct codec"),
            );
            let (tx, rx) = mpsc::unbounded_channel();
            let transport = Transport::new(framing, tx);
            let mut pinned = Box::pin(transport);
            pinned
                .send(Message {
                    prefix: Some("127.0.0.1".into()),
                    command: Command::Notice("*".to_owned(), "Welcome to 127.0.0.1".to_owned()),
                })
                .await
                .expect("Failed to send notify");
            while let Some(msg) = pinned.next().await {
                match msg {
                    Ok(msg) => println!("{:?}", msg),
                    Err(e) => eprintln!("{}", e),
                }
            }
        });
    }
}
