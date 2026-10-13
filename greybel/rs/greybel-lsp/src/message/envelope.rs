use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::{Message, Notification, Request, RequestId, Response};
use crate::error::ResponseError;

#[derive(Serialize, Deserialize)]
struct Envelope {
    jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<RequestId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    #[serde(
        default,
        deserialize_with = "Envelope::present_value",
        skip_serializing_if = "Option::is_none"
    )]
    result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<ResponseError>,
}

impl Envelope {
    const VERSION: &str = "2.0";

    fn present_value<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Value>, D::Error> {
        Value::deserialize(deserializer).map(Some)
    }

    fn empty() -> Self {
        Self {
            jsonrpc: Self::VERSION.to_owned(),
            id: None,
            method: None,
            params: None,
            result: None,
            error: None,
        }
    }

    fn params(params: &Value) -> Option<Value> {
        (!params.is_null()).then(|| params.clone())
    }

    fn into_message<E: serde::de::Error>(self) -> Result<Message, E> {
        let params = self.params.unwrap_or(Value::Null);

        match (self.method, self.id) {
            (Some(method), Some(id)) => Ok(Message::Request(Request { id, method, params })),
            (Some(method), None) => Ok(Message::Notification(Notification { method, params })),
            (None, id) => {
                let outcome = match (self.result, self.error) {
                    (_, Some(error)) => Err(error),
                    (Some(result), None) => Ok(result),
                    (None, None) => {
                        return Err(E::custom(
                            "a message without a method needs a result or an error",
                        ));
                    }
                };

                Ok(Message::Response(Response { id, outcome }))
            }
        }
    }
}

impl From<&Message> for Envelope {
    fn from(message: &Message) -> Self {
        match message {
            Message::Request(request) => Self {
                id: Some(request.id.clone()),
                method: Some(request.method.clone()),
                params: Self::params(&request.params),
                ..Self::empty()
            },
            Message::Notification(notification) => Self {
                method: Some(notification.method.clone()),
                params: Self::params(&notification.params),
                ..Self::empty()
            },
            Message::Response(response) => match &response.outcome {
                Ok(result) => Self {
                    id: response.id.clone(),
                    result: Some(result.clone()),
                    ..Self::empty()
                },
                Err(error) => Self {
                    id: response.id.clone(),
                    error: Some(error.clone()),
                    ..Self::empty()
                },
            },
        }
    }
}

impl Serialize for Message {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let envelope = Envelope::from(self);

        if let Self::Response(Response { id: None, .. }) = self {
            let mut value = serde_json::to_value(&envelope).map_err(serde::ser::Error::custom)?;

            value["id"] = Value::Null;

            return value.serialize(serializer);
        }

        envelope.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Message {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Envelope::deserialize(deserializer)?.into_message::<D::Error>()
    }
}
