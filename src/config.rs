use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct TLSCert {
    pub cert: String,
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Listener {
    pub name: String,
    pub address: SocketAddr,
    pub tls: Option<TLSCert>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Server {
    pub name: String,
    pub listeners: Vec<Listener>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub server: Server,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: Server {
                name: "localhost".to_string(),
                listeners: vec![Listener {
                    name: "plain".to_owned(),
                    address: "127.0.0.1:6667".parse::<SocketAddr>().unwrap(),
                    tls: None,
                }],
            },
        }
    }
}

impl Config {
    pub fn new(config: &Path) -> Result<Config, figment::Error> {
        let config: Config = Figment::from(Serialized::defaults(Config::default()))
            .merge(Toml::file(config))
            .merge(Env::prefixed("CAW_"))
            .extract()?;
        Ok(config)
    }
}
