use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Message {
    #[serde(rename = "subscribe")]
    Subscribe {
        #[serde(rename = "type")]
        sub_type: String,
        id: String,
    },
    #[serde(rename = "unsubscribe")]
    Unsubscribe {
        #[serde(rename = "type")]
        sub_type: String,
        id: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Subscription {
    pub sub_type: String,
    pub id: String,
}