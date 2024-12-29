use tokio_tungstenite::tungstenite::Message as WsMessage;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

#[derive(Clone)]
pub struct Session {
    pub id: Uuid,
    sender: futures::channel::mpsc::UnboundedSender<WsMessage>,
}

impl Session {
    pub fn new(id: Uuid, sender: futures::channel::mpsc::UnboundedSender<WsMessage>) -> Self {
        Session { id, sender }
    }

    pub fn send(&self, message: WsMessage) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self.sender.unbounded_send(message)?)
    }
}

impl PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Session {}

impl Hash for Session {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
} 