use std::{sync::Arc, collections::HashMap, hash::Hash};
use crate::Client;

pub struct Server {
    clients: HashMap<String, Client>
}

impl Server {
    pub fn new() -> Server {
        Self {
            clients: HashMap::new()
        }
    }

    pub async fn run() {

    }
}
