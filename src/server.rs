use crate::{tls_socket::Socket, Client};
use futures_util::stream::FuturesUnordered;
use futures_util::{StreamExt, TryFutureExt};
use proto::message::{Message, MessageContents};
use std::io;
use std::sync::Arc;
use std::{collections::HashMap, net::SocketAddr};
use thiserror::Error;
use tokio::{net::TcpListener, net::TcpStream};
use tokio_native_tls::native_tls;
use tokio_native_tls::TlsAcceptor;
use trust_dns_resolver::TokioAsyncResolver;

use proto::command::Command;
use proto::error::ProtocolError;
use proto::response::Response;

macro_rules! break_err {
    ($res:expr) => {
        match $res {
            Ok(val) => val,
            Err(e) => {
                break Err(e);
            }
        }
    };
}

#[derive(Debug)]
pub enum Listener {
    Tls(TcpListener, TlsAcceptor),
    Plain(TcpListener),
}

impl Listener {
    pub async fn new(addr: SocketAddr) -> Result<Listener, io::Error> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Listener::Plain(listener))
    }

    pub async fn new_tls(addr: SocketAddr, tls: TlsAcceptor) -> Result<Listener, io::Error> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Listener::Tls(listener, tls))
    }

    pub async fn accept(&self) -> Result<Socket<TcpStream>, ListenerError> {
        match self {
            Listener::Tls(ref listener, ref acceptor) => {
                let (socket, _) =
                    listener
                        .accept()
                        .await
                        .map_err(|e| ListenerError::ConnectionError {
                            string: "error accepting tcp connection".to_owned(),
                            cause: e,
                        })?;
                let stream =
                    acceptor
                        .accept(socket)
                        .await
                        .map_err(|e| ListenerError::TlsError {
                            string: "error in tls connection".to_owned(),
                            cause: e,
                        })?;
                Ok(Socket::Tls(stream))
            }
            Listener::Plain(listener) => {
                let (socket, _) =
                    listener
                        .accept()
                        .await
                        .map_err(|e| ListenerError::ConnectionError {
                            string: "error accepting tcp connection".to_owned(),
                            cause: e,
                        })?;
                Ok(Socket::Plain(socket))
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum ListenerError {
    #[error("connection error: {}", string)]
    ConnectionError {
        string: String,
        #[source]
        cause: io::Error,
    },
    #[error("tls error: {}", string)]
    TlsError {
        string: String,
        #[source]
        cause: native_tls::Error,
    },
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
enum ServerPhase {
    Startup,
    Running,
}

#[derive(Debug, Clone)]
pub struct ServerState {
    hostname: String,
}

impl ServerState {
    pub fn get_name(&self) -> &str {
        &self.hostname
    }
}

#[derive(Debug)]
pub struct Server {
    clients: HashMap<String, Client>,
    state: Arc<ServerState>,
    resolver: TokioAsyncResolver,
    listeners: Vec<Listener>,
    phase: ServerPhase,
}

impl Server {
    pub async fn new(hostname: String) -> Result<Server, ServerError> {
        let resolver =
            TokioAsyncResolver::tokio_from_system_conf().expect("Failed to create DNS resolver");
        Ok(Self {
            clients: HashMap::new(),
            resolver,
            listeners: Vec::new(),
            state: Arc::new(ServerState { hostname }),
            phase: ServerPhase::Startup,
        })
    }

    pub async fn add_tls_listener(
        &mut self,
        addr: SocketAddr,
        tls: TlsAcceptor,
    ) -> Result<(), ServerError> {
        if self.phase != ServerPhase::Startup {
            return Err(ServerError::ListenerModification(
                "attempt to add listener whilst running".to_owned(),
            ));
        }
        let listener = Listener::new_tls(addr, tls)
            .map_err(|e| ServerError::Io(e))
            .await?;
        self.listeners.push(listener);
        Ok(())
    }

    pub async fn add_listener(&mut self, addr: SocketAddr) -> Result<(), ServerError> {
        if self.phase != ServerPhase::Startup {
            return Err(ServerError::ListenerModification(
                "attempt to add listener whilst running".to_owned(),
            ));
        }
        let listener = Listener::new(addr).map_err(|e| ServerError::Io(e)).await?;
        self.listeners.push(listener);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), ServerError> {
        self.phase = ServerPhase::Running;
        loop {
            let mut iter: FuturesUnordered<_> = self.listeners.iter().map(|l| l.accept()).collect();
            let conn = loop {
                if let Some(c) = iter.next().await {
                    match c {
                        Ok(val) => break val,
                        Err(e) => eprintln!("{}", e),
                    }
                }
            };
            let resolver = self.resolver.clone();
            let ss = self.state.clone();
            tokio::spawn(async move {
                let mut client = Client::new(ss, conn)
                    .await
                    .expect("Client construction failed");
                client
                    .send(Command::Notice(
                        "*",
                        "*** Attempting lookup of your hostname...",
                    ))
                    .await
                    .expect("Failed to send message");
                match resolver.reverse_lookup(client.address().ip()).await {
                    Ok(val) => {
                        let hostname = val
                            .iter()
                            .nth(0)
                            .expect("Failed to get hostname even though i did");
                        client
                            .send(Command::Notice(
                                "*".to_owned(),
                                format!("*** Found hostname using {}", hostname.to_string()),
                            ))
                            .await
                            .expect("Failed to send message");
                    }
                    Err(e) => {
                        client.send(Command::Notice("*".to_owned(), format!("*** Lookup of hostname failed: {} using your ip address ({}) instead", e, client.address().ip()))).await.expect("Failed to send message");
                    }
                }
                client.poll_send().await.expect("Failed to send message");
                println!("Entering registration loop");
                let mut stream = client.stream().expect("Failed to obtain client stream.");
                let result: Result<(), ProtocolError> = loop {
                    if let Some(message) = stream.next().await {
                        if message.is_err() {
                            break Err(message.unwrap_err());
                        }
                        let message = message.unwrap();
                        println!("Message: {:?}", message);
                        match &message.contents {
                            MessageContents::Command(command) => match command {
                                _ => {
                                    break_err!(client.send(Response::ErrNotRegistered).await);
                                }
                            },
                            _ => (),
                        }
                    } else {
                        break Err(ProtocolError::PingTimeout);
                    }
                };
                if result.is_err() {
                    eprintln!("Error: {}", result.unwrap_err());
                    return;
                }
            });
        }
        //Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("listener modification error: {0}")]
    ListenerModification(String),
    #[error("tls error: {}", string)]
    Tls {
        string: String,
        #[source]
        cause: Option<native_tls::Error>,
    },
    #[error("IO error {0}")]
    Io(#[source] io::Error),
}
