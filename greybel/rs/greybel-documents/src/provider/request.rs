use std::io;

use vscode_uri::Uri;

use super::join::JoinAll;
use super::{AsyncResourceProvider, ResourceProvider};
use crate::error::{DocumentError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceRequest {
    RelativeTo { source: Uri, target: String },
    Read(Uri),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceResponse {
    Uri(Uri),
    Content(Option<String>),
}

impl ResourceRequest {
    pub fn run(self, provider: &impl ResourceProvider) -> Result<ResourceResponse> {
        Ok(match self {
            Self::RelativeTo { source, target } => {
                ResourceResponse::Uri(provider.relative_to(&source, &target))
            }
            Self::Read(target) => match provider.read(&target) {
                Ok(content) => ResourceResponse::Content(content),
                Err(error) => return Err(Self::read_error(target, error)),
            },
        })
    }

    pub async fn run_async(
        self,
        provider: &impl AsyncResourceProvider,
    ) -> Result<ResourceResponse> {
        Ok(match self {
            Self::RelativeTo { source, target } => {
                match provider.relative_to(&source, &target).await {
                    Ok(uri) => ResourceResponse::Uri(uri),
                    Err(error) => return Err(Self::resolve_error(target, error)),
                }
            }
            Self::Read(target) => match provider.read(&target).await {
                Ok(content) => ResourceResponse::Content(content),
                Err(error) => return Err(Self::read_error(target, error)),
            },
        })
    }

    pub(crate) fn run_all(
        requests: Vec<Self>,
        provider: &impl ResourceProvider,
    ) -> Result<Vec<ResourceResponse>> {
        requests
            .into_iter()
            .map(|request| request.run(provider))
            .collect()
    }

    pub(crate) async fn run_all_async(
        requests: Vec<Self>,
        provider: &impl AsyncResourceProvider,
    ) -> Result<Vec<ResourceResponse>> {
        JoinAll::new(
            requests
                .into_iter()
                .map(|request| request.run_async(provider)),
        )
        .await
        .into_iter()
        .collect()
    }

    fn resolve_error(target: String, source: io::Error) -> DocumentError {
        DocumentError::Resolve { target, source }
    }

    fn read_error(target: Uri, source: io::Error) -> DocumentError {
        DocumentError::Read { target, source }
    }
}
