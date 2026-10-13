use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::error::ReferenceError;
use super::files::{copy_folder, replace_folder, write_json};
use crate::asset_record::fingerprints_of_records;

#[derive(Debug)]
pub struct Capture {
    pub(super) folder: PathBuf,
    pub(super) sources: Vec<String>,
    pub(super) index: Value,
}

impl Capture {
    /// Serializes TMP layouts into text-mesh-layout fixture format.
    ///
    /// # Errors
    /// Returns an error if `path` cannot be written.
    pub fn write_layout(&self, path: &Path) -> Result<(), ReferenceError> {
        write_json(
            &self.cases(|case, capture| {
                let items: Vec<Value> = items(capture)
                    .iter()
                    .map(|item| {
                        let mut item = item.as_object().cloned().unwrap_or_default();

                        item.remove("meshes");
                        item.remove("lossyScale");

                        Value::Object(item)
                    })
                    .collect();

                case.insert("items".to_owned(), Value::Array(items));
            }),
            path,
        )
    }

    /// Serializes TMP meshes and captures into text-mesh-webgl fixture format.
    ///
    /// # Errors
    /// Returns an error if `path` cannot be written.
    pub fn write_meshes(&self, path: &Path) -> Result<(), ReferenceError> {
        write_json(
            &self.cases(|case, capture| {
                case.insert("image".to_owned(), capture["file"].clone());

                let items: Vec<Value> = items(capture)
                    .iter()
                    .map(|item| {
                        json!({
                            "top": item["top"],
                            "lossyScale": item["lossyScale"],
                            "meshes": item["meshes"],
                        })
                    })
                    .collect();

                case.insert("items".to_owned(), Value::Array(items));
            }),
            path,
        )
    }

    /// Copies captured `line-NNN.png` images into an emptied `folder`.
    ///
    /// # Errors
    /// Returns an error if an image copy fails.
    pub fn copy_images(&self, folder: &Path) -> Result<(), ReferenceError> {
        replace_folder(folder)?;

        copy_folder(&self.folder, folder, &|name| {
            name.starts_with("line-") && name.ends_with(".png")
        })
    }

    /// Writes fingerprints for the assets used by the captured meshes.
    ///
    /// # Errors
    /// When a record can't be read or `path` can't be written.
    pub fn write_asset_fingerprints(&self, path: &Path) -> Result<(), ReferenceError> {
        let fingerprints = fingerprints_of_records(&self.folder.join("assets"))?;

        write_json(&serde_json::to_value(fingerprints)?, path)
    }

    fn cases(&self, add: impl Fn(&mut Map<String, Value>, &Value)) -> Value {
        let cases: Vec<Value> = self.index["captures"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(|capture| {
                let line = capture["line"]
                    .as_u64()
                    .and_then(|line| usize::try_from(line).ok());
                let source = line.and_then(|line| self.sources.get(line)).cloned();
                let mut case = Map::new();

                case.insert(
                    "source".to_owned(),
                    Value::String(source.unwrap_or_default()),
                );
                case.insert("height".to_owned(), capture["height"].clone());
                add(&mut case, capture);

                Value::Object(case)
            })
            .collect();

        json!({ "width": self.index["width"], "cases": cases })
    }
}

fn items(capture: &Value) -> &[Value] {
    capture["items"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
}
