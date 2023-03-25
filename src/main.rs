use std::fs::read;
use tokio_native_tls::TlsAcceptor;
use client::Client;
use server::Server;
use tokio_native_tls::native_tls::Identity;
use config::Config;
use std::path::Path;
mod client;
mod connection;
mod server;
mod config;
mod tls_socket;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conf = Config::new(Path::new("config.toml")).expect("Failed to read config");
    println!("{:?}", conf);
    let mut server = Server::new(conf.server.name).await?;
    for listener in conf.server.listeners {
        if let Some(tls) = listener.tls {
            let cert = read(&tls.cert).expect(format!("Failed to read TLS certificate {}", &tls.cert).as_ref());
            let key = read(&tls.key).expect(format!("Failed to read TLS key {}", &tls.key).as_ref());
            let ident = Identity::from_pkcs8(&cert, &key).expect("Failed to construct certificate identity");
            let acceptor = TlsAcceptor::from(tokio_native_tls::native_tls::TlsAcceptor::builder(ident).build()?);
            server.add_tls_listener(listener.address, acceptor).await.expect("Failed to create TLS listener");
        } else {
            server.add_listener(listener.address).await.expect("Failed to create plain listener");
        }
    }
    server.run().await;
    Ok(())
}
