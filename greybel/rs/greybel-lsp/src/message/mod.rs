mod envelope;
mod notification;
mod request;
mod response;

pub use notification::Notification;
pub use request::Request;
pub use response::Response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}
