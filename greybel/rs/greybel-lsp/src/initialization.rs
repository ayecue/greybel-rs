use greybel_core::Dialect;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ResponseError, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitializationOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialect: Option<String>,
}

impl InitializationOptions {
    pub fn of(value: Option<Value>) -> Result<Self> {
        match value {
            None | Some(Value::Null) => Ok(Self::default()),
            Some(value) => Self::deserialize(value).map_err(|error| {
                ResponseError::invalid_params(format!("Invalid initialization options: {error}"))
            }),
        }
    }

    pub fn dialect(&self) -> Result<Dialect> {
        let Some(name) = &self.dialect else {
            return Ok(Dialect::GreyScript);
        };

        Dialect::ALL
            .into_iter()
            .find(|dialect| dialect.name() == name)
            .ok_or_else(|| ResponseError::invalid_params(format!("Unknown dialect: {name}")))
    }
}
