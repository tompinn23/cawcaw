use proto::error::{self, ProtocolError};
use proto::message::Message;
use tokio::sync::mpsc::UnboundedSender;

pub struct Sender {
    tx: UnboundedSender<Message>,
}

impl Sender {
    pub fn send<M: Into<Message>>(&self, msg: M) -> error::Result<()> {
        self.tx
            .send(msg.into())
            .map_err(|e| ProtocolError::SendError(e))
    }
}
