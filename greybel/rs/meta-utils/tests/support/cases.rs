use meta_utils::TypeMeta;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
pub struct Case<'i> {
    pub input: &'i Value,
    pub output: Value,
}

pub fn inputs(name: &str) -> Vec<Value> {
    let inputs: Value =
        serde_json::from_str(include_str!("../fixtures/inputs.json")).expect("valid inputs");

    inputs[name].as_array().expect("a list of inputs").clone()
}

pub fn outcome<E: std::fmt::Display>(parsed: Result<TypeMeta, E>) -> Value {
    match parsed {
        Ok(meta) => json!({ "string": meta.to_string(), "json": meta }),
        Err(error) => json!({ "error": error.to_string() }),
    }
}
