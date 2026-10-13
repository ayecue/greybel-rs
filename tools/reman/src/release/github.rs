//! GitHub repository releases are published to, through its HTTP API (`api.github.com`): its tags, and creating a tag with its
//! release. A real run needs a token (`GITHUB_TOKEN`, else `GH_TOKEN`); reading tags works without one.

use anyhow::{Context, Result};
use octocrab::Octocrab;
use octocrab::params::repos::Reference;
use octocrab::repos::releases::MakeLatest;
use serde::Deserialize;
use serde_json::json;
use tokio::runtime::Runtime;

const TOKEN_VARIABLES: [&str; 2] = ["GITHUB_TOKEN", "GH_TOKEN"];

pub struct GitHub {
    client: Octocrab,
    runtime: Runtime,
    owner: String,
    name: String,
}

/// Release to create.
pub struct Release<'a> {
    pub tag: &'a str,
    /// Commit the tag names.
    pub commit: &'a str,
    pub title: &'a str,
    pub notes: &'a str,
    pub draft: bool,
    /// Marks the repository's latest release.
    pub latest: bool,
}

#[derive(Deserialize)]
struct TagObject {
    sha: String,
}

impl GitHub {
    /// Repository `owner/name`, authenticated by the first token variable set.
    pub fn connect(repository: &str) -> Result<Self> {
        let (owner, name) = repository
            .split_once('/')
            .with_context(|| format!("`{repository}` is not `owner/name`"))?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let token = TOKEN_VARIABLES
            .iter()
            .find_map(|variable| std::env::var(variable).ok());
        let client = {
            let _context = runtime.enter();
            let builder = Octocrab::builder();

            match token {
                Some(token) => builder.personal_token(token).build()?,
                None => builder.build()?,
            }
        };

        Ok(GitHub {
            client,
            runtime,
            owner: owner.to_owned(),
            name: name.to_owned(),
        })
    }

    pub fn has_token() -> bool {
        TOKEN_VARIABLES
            .iter()
            .any(|variable| std::env::var_os(variable).is_some())
    }

    pub fn has_tag(&self, tag: &str) -> Result<bool> {
        let reference = Reference::Tag(tag.to_owned());
        let response = self.runtime.block_on(
            self.client
                .repos(&self.owner, &self.name)
                .get_ref(&reference),
        );

        match response {
            Ok(_) => Ok(true),
            Err(octocrab::Error::GitHub { source, .. }) if source.status_code == 404 => Ok(false),
            Err(error) => Err(error).with_context(|| format!("reading the tag {tag}")),
        }
    }

    /// Creates an annotated tag with the title as its message before creating the release, ensuring draft releases also have tags.
    pub fn create_release(&self, release: &Release<'_>) -> Result<()> {
        let repository = self.client.repos(&self.owner, &self.name);
        let tag_route = format!("/repos/{}/{}/git/tags", self.owner, self.name);
        let tag_body = json!({
            "tag": release.tag,
            "message": release.title,
            "object": release.commit,
            "type": "commit",
        });
        let latest = if release.latest {
            MakeLatest::True
        } else {
            MakeLatest::False
        };

        self.runtime.block_on(async {
            let tag: TagObject = self.client.post(tag_route, Some(&tag_body)).await?;

            repository
                .create_ref(&Reference::Tag(release.tag.to_owned()), tag.sha)
                .await?;
            repository
                .releases()
                .create(release.tag)
                .name(release.title)
                .body(release.notes)
                .draft(release.draft)
                .make_latest(latest)
                .send()
                .await?;

            Ok::<_, octocrab::Error>(())
        })?;

        Ok(())
    }
}
