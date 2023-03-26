use crate::client::ClientState;
use crate::config::Config;
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
use tokio::sync::RwLock;

use proto::command::Command;
use proto::error::{ProtocolError, MessageParseError};
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
    clients: Arc<RwLock<HashMap<String, Arc<RwLock<Client>>>>>,
}

impl ServerState {

    pub fn new(hostname: String) -> Self {
        Self {
            hostname,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.hostname
    }

    pub async fn send<M: Into<Message>>(&self, client: &Client, msg: M) -> Result<(), ProtocolError> {
        let mut msg: Message = msg.into();
        msg.set_prefix(&self.get_name());
        client.sender().send(msg)
    }

    pub async fn check_nick(&self, nick: &str) -> bool {
        self.clients.read().await.contains_key(nick)
    }

    pub async fn register_client(&mut self, mut client: Client, password: Option<String>, nick: &String, user: &String, real: &String) -> Result<Arc<RwLock<Client>>, Response> {
        let mut clients = self.clients.write().await;
        client.register(&nick, &user, &real);
        let client = Arc::new(RwLock::new(client));
        clients.insert(nick.to_string(), client.clone());
        Ok(client)
    }
}

#[derive(Debug)]
pub struct Server {
    state: Arc<RwLock<ServerState>>,
    resolver: TokioAsyncResolver,
    listeners: Vec<Listener>,
    phase: ServerPhase,
}

impl Server {
    pub async fn new(hostname: String) -> Result<Server, ServerError> {
        let resolver =
            TokioAsyncResolver::tokio_from_system_conf().expect("Failed to create DNS resolver");
        Ok(Self {
            resolver,
            listeners: Vec::new(),
            state: Arc::new(RwLock::new(ServerState::new(hostname))),
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

    pub fn set_phase(&mut self, phase: ServerPhase) {
        self.phase = phase;
    }

    pub async fn wait_for_client(&mut self) -> Result<Socket<TcpStream>, ServerError> {
        let mut iter: FuturesUnordered<_> = self.listeners.iter().map(|l| l.accept()).collect();
        let conn = loop {
            if let Some(c) = iter.next().await {
                match c {
                    Ok(val) => break val,
                    Err(e) => eprintln!("{}", e),
                }
            }
        };
        Ok(conn)
    }

    pub async fn run(&mut self) -> Result<(), ServerError> {
        self.phase = ServerPhase::Running;
        loop {
            let conn = self.wait_for_client().await.expect("Error accepting client");
            let resolver = self.resolver.clone();
            let server = self.state.clone();
            tokio::spawn(async move {
                let mut client = Client::new(conn)
                    .await
                    .expect("Client construction failed");
                server.read().await.send(&client, Command::Notice(
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
                        server.read().await.send(&client, Command::Notice(
                                "*".to_owned(),
                                format!("*** Found hostname using {}", hostname.to_string()),
                            ))
                            .await
                            .expect("Failed to send message");
                    }
                    Err(e) => {
                        server.read().await.send(&client, Command::Notice("*".to_owned(), format!("*** Lookup of hostname failed: {} using your ip address ({}) instead", e, client.address().ip()))).await.expect("Failed to send message");
                    }
                }
                client.poll_send().await.expect("Failed to send message");
                println!("Entering registration loop");
                let mut stream = client.stream().expect("Failed to obtain client stream.");
                let mut password: Option<String> = None;
                let mut nick = String::new();
                let result: Result<Arc<RwLock<Client>>, ProtocolError> = loop {
                    if let Some(message) = stream.next().await {
                        match message {
                            Ok(message) => {
                                println!("Message: {:?}", message);
                                match &message.contents {
                                    MessageContents::Command(command) => match command {
                                        Command::PASS(pass) => {
                                            password = Some(pass.to_owned());
                                        }
                                        Command::NICK(nickname, _) => {
                                            if server.read().await.check_nick(nickname).await {
                                                break_err!(server.read().await.send(&client, Response::ErrNickCollision(nickname.clone())).await);
                                            }
                                            nick = nickname.to_owned();
                                        }
                                        Command::USER(user, _, _, real) => {
                                            match server.write().await.register_client(client, password, &nick, user, real).await {
                                                Ok(v) => break Ok(v),
                                                Err(e) => break Err(ProtocolError::ServerError)
                                            };
                                        }
                                        Command::PONG(_, _) | Command::PING(_, _) => {}
                                        _ => {
                                            break_err!(server.read().await.send(&client, Response::ErrNotRegistered).await);
                                        }
                                    },
                                    _ => (),
                                }
                            }
                            Err(e) => match e {
                                ProtocolError::InvalidMessage { string, cause } => match cause {
                                    MessageParseError::ErrResponse(r) => {
                                        break_err!(server.read().await.send(&client, r).await);
                                    }
                                    _ => break Err(ProtocolError::InvalidMessage { string, cause })
                                }
                                _ => break Err(e)
                            }
                        }
                    }
                };
                if result.is_err() {
                    eprintln!("Error: {}", result.unwrap_err());
                    return;
                }
                let client = result.unwrap();
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
