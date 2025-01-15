use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use futures::channel::mpsc::UnboundedSender;
use uuid::Uuid;
use crate::types::Subscription;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub sender: UnboundedSender<WsMessage>,
    pub account_id: Option<i32>,
    pub auth_token: Option<String>,
    pub subscriptions: HashSet<Subscription>,
}

impl Session {
    pub fn new(id: Uuid, sender: UnboundedSender<WsMessage>) -> Self {
        Session {
            id,
            sender,
            account_id: None,
            auth_token: None,
            subscriptions: HashSet::new(),
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

    pub fn add_subscription(&mut self, subscription: Subscription) {
        self.subscriptions.insert(subscription);
    }

    pub fn remove_subscription(&mut self, subscription: &Subscription) {
        self.subscriptions.remove(subscription);
    }

    pub fn has_subscription(&self, subscription: &Subscription) -> bool {
        self.subscriptions.contains(subscription)
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