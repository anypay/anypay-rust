use tokio_tungstenite::tungstenite::Message as WsMessage;
use std::hash::{Hash, Hasher};
use uuid::Uuid;
use futures::channel::mpsc::UnboundedSender;

#[derive(Clone)]
pub struct Session {
    pub id: Uuid,
    pub sender: UnboundedSender<WsMessage>,
    pub account_id: Option<i32>,
    pub auth_token: Option<String>,
}

impl Session {
    pub fn new(id: Uuid, sender: UnboundedSender<WsMessage>) -> Self {
        Self {
            id,
            sender,
            account_id: None,
            auth_token: None,
        }
    }

    pub fn set_account_id(&mut self, account_id: i32) {
        self.account_id = Some(account_id);
    }

    pub fn is_authorized(&self) -> bool {
        self.account_id.is_some()
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