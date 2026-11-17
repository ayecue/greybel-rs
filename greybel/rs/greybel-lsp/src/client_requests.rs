use std::collections::BTreeMap;

use gen_lsp_types::{LspRequestMethod, MessageDirection, Request};
use greybel_project::Manifest;
use serde::{Deserialize, Serialize};

/// Server request for a URI's file contents.
pub enum ReadFile {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadFileRequest {
    pub uri: String,
}

impl Request for ReadFile {
    type Params = ReadFileRequest;
    type Result = Option<String>;

    const MESSAGE_DIRECTION: MessageDirection = MessageDirection::ServerToClient;
    const METHOD: LspRequestMethod<'static> = LspRequestMethod::new("custom/read-file");
}

/// Server request for file URIs matching include and exclude globs.
pub enum FileSearch {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindFilesRequest {
    pub include: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<String>,
}

impl Request for FileSearch {
    type Params = FindFilesRequest;
    type Result = Vec<String>;

    const MESSAGE_DIRECTION: MessageDirection = MessageDirection::ServerToClient;
    const METHOD: LspRequestMethod<'static> = LspRequestMethod::new("custom/find-files");
}

/// Client request for the files a document imports through `#include`, `#include type`, `#import` and `import_code`,
/// directly or through other imported files, in the order they are imported. The document is not listed; missing
/// files are skipped.
pub enum ImportedFiles {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedFilesRequest {
    pub uri: String,
}

impl Request for ImportedFiles {
    type Params = ImportedFilesRequest;
    type Result = Vec<String>;

    const MESSAGE_DIRECTION: MessageDirection = MessageDirection::ClientToServer;
    const METHOD: LspRequestMethod<'static> = LspRequestMethod::new("custom/imported-files");
}

/// Client request for a document's project. Uses the nearest `greybel.json` at or above the document folder, or
/// `fallback`, then applies `profile` and loads values from variable files.
pub enum Project {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRequest {
    pub uri: String,
    /// Fallback settings for documents without `greybel.json`, such as editor settings; defaults apply when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<Manifest>,
    /// Project profile used for the result's build and run settings; omitted profiles select base settings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResult {
    /// Folder containing the project's `greybel.json`, when present.
    pub root: Option<String>,
    /// Complete project settings with the requested profile applied; `profiles` lists all profile names.
    pub manifest: Manifest,
    /// Values available to `#envar` during project builds.
    pub build_variables: BTreeMap<String, String>,
    /// Values available to `#envar` during project runs.
    pub run_variables: BTreeMap<String, String>,
}

impl Request for Project {
    type Params = ProjectRequest;
    type Result = ProjectResult;

    const MESSAGE_DIRECTION: MessageDirection = MessageDirection::ClientToServer;
    const METHOD: LspRequestMethod<'static> = LspRequestMethod::new("custom/project");
}
