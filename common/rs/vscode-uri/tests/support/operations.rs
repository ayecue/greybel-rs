use serde::Serialize;
use serde_json::Value;
use vscode_uri::{Result, Uri, UriChange, UriError};

pub type OnePath = fn(&str) -> String;

pub type ManyPaths = fn(&[&str]) -> String;

#[derive(Serialize)]
pub struct Case<'i, O> {
    pub input: &'i Value,
    pub output: O,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum Outcome {
    Uri(Fields),
    Error { error: &'static str },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fields {
    pub scheme: String,
    pub authority: String,
    pub path: String,
    pub query: String,
    pub fragment: String,
    pub string: String,
    pub unencoded: String,
    pub fs_path: String,
}

#[derive(Serialize)]
pub struct Names {
    pub basename: String,
    pub extname: String,
}

pub fn inputs() -> Value {
    serde_json::from_str(include_str!("../fixtures/inputs.json")).expect("valid inputs")
}

pub fn list<'i>(inputs: &'i Value, name: &str) -> &'i [Value] {
    inputs[name].as_array().expect("a list of inputs")
}

pub fn text(value: &Value) -> &str {
    value.as_str().expect("a string")
}

pub fn strings(value: &Value) -> Vec<&str> {
    value.as_array().expect("a list").iter().map(text).collect()
}

pub fn outcome(result: Result<Uri>) -> Outcome {
    match result {
        Ok(uri) => Outcome::Uri(Fields {
            scheme: uri.scheme().to_owned(),
            authority: uri.authority().to_owned(),
            path: uri.path().to_owned(),
            query: uri.query().to_owned(),
            fragment: uri.fragment().to_owned(),
            string: uri.to_string(),
            unencoded: uri.to_unencoded_string(),
            fs_path: uri.fs_path(),
        }),
        Err(error) => Outcome::Error {
            error: match error {
                UriError::MissingScheme { .. } => "MissingScheme",
                UriError::IllegalScheme => "IllegalScheme",
                UriError::RelativePath => "RelativePath",
                UriError::DoubleSlashPath => "DoubleSlashPath",
            },
        },
    }
}

/// Converts a JSON `with` change to `UriChange`: absent fields remain unchanged; `null` becomes an empty string.
pub fn change(value: &Value) -> UriChange {
    let field = |name: &str| match value.get(name) {
        None => None,
        Some(Value::Null) => Some(String::new()),
        Some(value) => Some(text(value).to_owned()),
    };

    UriChange {
        scheme: field("scheme"),
        authority: field("authority"),
        path: field("path"),
        query: field("query"),
        fragment: field("fragment"),
    }
}

pub fn base(input: &Value) -> Uri {
    Uri::parse(text(&input["base"])).expect("a base")
}

pub fn cases<'i, O>(inputs: &'i [Value], operation: impl Fn(&Value) -> O) -> Vec<Case<'i, O>> {
    inputs
        .iter()
        .map(|input| Case {
            input,
            output: operation(input),
        })
        .collect()
}
