use std::{fs::read, pin};

use client::Client;
use futures_util::SinkExt;
use futures_util::StreamExt;
use proto::{codecs::MessageCodec, command::Command, message::Message, transport::Transport};
use tokio::{net::TcpListener, sync::mpsc};
use tokio_native_tls::native_tls::{self, Identity};
use tokio_util::codec::Framed;
use trust_dns_resolver::TokioAsyncResolver;

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
    let resolver = TokioAsyncResolver::tokio_from_system_conf().expect("Failed to construct DNS resolver");
    loop {
        let (socket, _) = listener.accept().await?;
        let peer = socket.peer_addr().expect("Couldn't get peer addr");
        let tls_acceptor = tls_acceptor.clone();
        let resolver = resolver.clone();
        tokio::spawn(async move {
            let stream = tls_acceptor
                .accept(socket)
                .await
                .expect("Failed to accept tls connection");
            let mut client = Client::new_tls(stream).await.expect("Failed to construct new client");
            let sender = client.sender();
            sender.send(Message {
                prefix: Some("127.0.0.1".into()),
                command: Command::Notice("*".to_owned(), "*** Hello".to_owned())
            }).expect("failed to send notice");
            let mut stream = client.stream().expect("Failed to get client stream");
            sender.send(Message {
                prefix: Some("127.0.0.1".into()),
                command: Command::Notice("*".to_owned(), "*** Attempting lookup of your hostname...".to_owned())
            });
            client.pump_send().await.expect("cant pump the sender");
            match resolver.reverse_lookup(peer.ip()).await {
                Ok(v) => {
                    println!("found hostname: {:?}", v);
                    sender.send(Message{
                        prefix: Some("127.0.0.1".into()),
                        command: Command::Notice("*".to_owned(), format!("*** Found hostname: {}", v.iter().nth(0).unwrap()))
                    });
                }
                Err(e) => {
                    sender.send(Message{
                        prefix: Some("127.0.0.1".into()),
                        command: Command::Notice("*".to_owned(), format!("*** Failed to find hostname using your ip address instead ({})", peer.ip()))
                    });
                }
            }
            client.pump_send().await.expect("cant pump the sender");
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(m) => {
                        println!("{:?}", m);
                    }
                    Err(e) => panic!("{}", e)
                }
            }
        });
    }
}
