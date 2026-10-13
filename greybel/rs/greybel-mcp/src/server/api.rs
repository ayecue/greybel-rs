//! Implements `api_search` and `api_reference` for the GreyScript API.

use std::fmt::Write as _;

use greybel_api_reference::protocol::Search;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use super::Server;
use super::source::into_tool_contents;

#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("The API has no type {owner}: find one with api_search.")]
    UnknownType { owner: String },
    #[error("{owner} has no member {member}.")]
    UnknownMember { owner: String, member: String },
}

into_tool_contents!(ApiError);

/// Fence language for reference code.
const LANGUAGE: &str = "greyscript";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ApiSearchRequest {
    /// Case-insensitive text filter for type names or member `type.member` names.
    pub query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ApiReferenceRequest {
    /// Type name such as `shell`, or member name such as `shell.host_computer`.
    pub name: String,
}

#[tool_router(router = api_router, vis = "pub(super)")]
impl Server {
    #[tool(
        description = "Finds types and members of the GreyScript API whose names contain the query. Read one with api_reference."
    )]
    async fn api_search(&self, Parameters(params): Parameters<ApiSearchRequest>) -> String {
        let result = greybel_api_reference::search(&Search {
            query: params.query.into(),
            excluded_tags: Vec::new().into(),
        });
        let mut text = String::new();

        for name in &result.types {
            let _ = writeln!(text, "type {name}");
        }

        for path in &result.members {
            let _ = writeln!(text, "{}.{}", path.owner, path.name);
        }

        if text.is_empty() {
            "Nothing matches.".to_owned()
        } else {
            text
        }
    }

    #[tool(
        description = "The reference of a type (such as `shell`) or a member (such as `shell.host_computer`) of the GreyScript API, as Markdown: signatures, descriptions, examples."
    )]
    async fn api_reference(
        &self,
        Parameters(params): Parameters<ApiReferenceRequest>,
    ) -> Result<String, ApiError> {
        let (_, sections) = greybel_api_reference::reference();
        let (owner, member) = match params.name.split_once('.') {
            Some((owner, member)) => (owner, Some(member)),
            None => (params.name.as_str(), None),
        };
        let section = sections
            .iter()
            .find(|section| section.name.eq_ignore_ascii_case(owner))
            .ok_or_else(|| ApiError::UnknownType {
                owner: owner.to_owned(),
            })?;

        match member {
            None => Ok(section.markdown(LANGUAGE)),
            Some(member) => section
                .members
                .iter()
                .find(|candidate| candidate.name == member)
                .map(|member| member.markdown(&section.name, LANGUAGE))
                .ok_or_else(|| ApiError::UnknownMember {
                    owner: section.name.clone(),
                    member: member.to_owned(),
                }),
        }
    }
}
