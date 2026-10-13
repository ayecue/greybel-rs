//! greybel.json's schema: the greybel-vs settings it names.

use serde_json::Value;

/// Every `x-vscode-setting` under `schema`, with its format and the default of its setting in `defaults`.
pub fn vscode_settings(
    root: &Value,
    schema: &Value,
    defaults: &Value,
) -> Vec<(String, Option<String>, Value)> {
    let mut settings = Vec::new();
    let Some(properties) = schema["properties"].as_object() else {
        return settings;
    };

    for (name, property) in properties {
        let default = &defaults[name];
        let definition = definition(root, property);
        let format = property["x-vscode-format"].as_str().map(str::to_owned);

        match &property["x-vscode-setting"] {
            Value::String(setting) => settings.push((setting.clone(), format, default.clone())),
            Value::Object(parts) => {
                for (part, setting) in parts {
                    let format = definition["properties"][part]["x-vscode-format"]
                        .as_str()
                        .map(str::to_owned);

                    settings.push((
                        setting.as_str().expect("a setting's name").to_owned(),
                        format,
                        default[part].clone(),
                    ));
                }
            }
            _ => settings.extend(vscode_settings(root, definition, default)),
        }
    }

    settings
}

/// The definition `property` refers to, or `property` itself.
fn definition<'s>(root: &'s Value, property: &'s Value) -> &'s Value {
    property["$ref"]
        .as_str()
        .and_then(|reference| reference.strip_prefix("#/$defs/"))
        .map_or(property, |name| &root["$defs"][name])
}

/// `value` as greybel-vs writes it in a setting of `format`.
pub fn vscode_value(value: &Value, format: Option<&str>) -> Value {
    let items = || {
        value
            .as_array()
            .expect("a list")
            .iter()
            .map(|item| item.as_str().expect("a string"))
    };

    match format {
        Some("commaList") => Value::String(items().collect::<Vec<_>>().join(",")),
        Some("spaceList") => Value::String(items().collect::<Vec<_>>().join(" ")),
        Some("first") => items()
            .next()
            .map_or(Value::Null, |item| Value::String(item.to_owned())),
        _ => value.clone(),
    }
}
