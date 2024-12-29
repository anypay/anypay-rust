use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;
use crate::session::Session;
use crate::types::Subscription;

pub struct EventDispatcher {
    subscriptions: RwLock<HashMap<Subscription, HashSet<uuid::Uuid>>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        EventDispatcher {
            subscriptions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn subscribe(&self, session: Session, sub_type: &str, id: &str) {
        let subscription = Subscription {
            sub_type: sub_type.to_string(),
            id: id.to_string(),
        };
        
        let mut subs = self.subscriptions.write().await;
        subs.entry(subscription)
            .or_insert_with(HashSet::new)
            .insert(session.id);
    }

    pub async fn unsubscribe(&self, session: Session, sub_type: &str, id: &str) {
        let subscription = Subscription {
            sub_type: sub_type.to_string(),
            id: id.to_string(),
        };
        
        let mut subs = self.subscriptions.write().await;
        if let Some(sessions) = subs.get_mut(&subscription) {
            sessions.remove(&session.id);
            if sessions.is_empty() {
                subs.remove(&subscription);
            }
        }
    }
} 