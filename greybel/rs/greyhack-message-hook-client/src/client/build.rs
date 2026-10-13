use std::time::Duration;

use duplex_core::{Bytes, Message};

use super::{Client, ClientError, Subscription};
use crate::protocol::greyhack::{
    AddResourceToBuildRpc, AddScriptToBuildRpc, BuildFinishedRpc, BuildStateClientRpc,
    BuildStateServerRpc, CreateBuildRpc, DisposeBuildRpc, DisposedBuildRpc, Greybel,
    PerformBuildRpc,
};
use crate::protocol::{BuildState, GreybelNotification, Payload, ScriptType};

#[derive(Debug, Clone, PartialEq)]
pub enum BuildOutcome {
    Completion(BuildFinishedRpc<'static>),
    Disposal(DisposedBuildRpc<'static>),
}

impl BuildOutcome {
    #[must_use]
    pub fn state(&self) -> BuildState {
        BuildState::from(match self {
            Self::Completion(m) => m.state,
            Self::Disposal(m) => m.state,
        })
    }

    #[must_use]
    pub fn output(&self) -> String {
        match self {
            Self::Completion(m) => m.output.text(),
            Self::Disposal(m) => m.output.text(),
        }
    }

    #[must_use]
    pub fn error_message(&self) -> &str {
        match self {
            Self::Completion(m) => &m.error_message,
            Self::Disposal(m) => &m.error_message,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Build {
    client: Client,
    build_id: String,
}

impl Build {
    pub(super) async fn create(
        client: Client,
        destination: &str,
        resource_destination: &str,
        auto_compile: bool,
    ) -> Result<Self, ClientError> {
        let build_id = client.new_id();
        let request = CreateBuildRpc {
            build_id: build_id.clone().into(),
            destination: destination.to_owned().into(),
            resource_destination: resource_destination.to_owned().into(),
            auto_compile,
        };
        let target_id = build_id.clone();
        let notification = client
            .request(
                &request,
                move |n| matches!(n, GreybelNotification::CreatedBuildRpc(m) if m.build_id == target_id),
                None,
            )
            .await?;
        let GreybelNotification::CreatedBuildRpc(reply) = notification else {
            return Err(ClientError::UnexpectedResponse(notification.name()));
        };

        Ok(Self {
            client,
            build_id: reply.build_id.into_owned(),
        })
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.build_id
    }

    #[must_use]
    pub fn events(&self) -> Subscription {
        let id = self.build_id.clone();

        self.client.subscribe(move |n| n.belongs_to_build(&id))
    }

    async fn request<M>(
        &self,
        request: M,
        predicate: fn(&GreybelNotification) -> bool,
    ) -> Result<GreybelNotification, ClientError>
    where
        M: Message<Sender = Greybel> + Clone + Send + 'static,
    {
        let id = self.build_id.clone();

        self.client
            .request(
                &request,
                move |n| predicate(n) && n.belongs_to_build(&id),
                None,
            )
            .await
    }

    pub async fn add_resource(&self, location: &str, content: &str) -> Result<(), ClientError> {
        let request = AddResourceToBuildRpc {
            build_id: self.build_id.clone().into(),
            location: location.to_owned().into(),
            zip_content: Bytes::from_text(content),
        };

        self.request(request, |n| {
            matches!(n, GreybelNotification::AddedResourceToBuildRpc(_))
        })
        .await
        .map(drop)
    }

    pub async fn add_script(
        &self,
        script_type: ScriptType,
        content: &str,
    ) -> Result<(), ClientError> {
        let request = AddScriptToBuildRpc {
            build_id: self.build_id.clone().into(),
            r#type: script_type.into(),
            zip_content: Bytes::from_text(content),
        };

        self.request(request, |n| {
            matches!(n, GreybelNotification::AddedScriptToBuildRpc(_))
        })
        .await
        .map(drop)
    }

    pub async fn perform(&self) -> Result<(), ClientError> {
        self.client
            .send(&PerformBuildRpc {
                build_id: self.build_id.clone().into(),
            })
            .await
    }

    pub async fn perform_and_wait(
        &self,
        timeout: Option<Duration>,
    ) -> Result<BuildOutcome, ClientError> {
        let id = self.build_id.clone();
        let mut subscription = self.client.subscribe(move |n| {
            matches!(
                n,
                GreybelNotification::BuildFinishedRpc(_) | GreybelNotification::DisposedBuildRpc(_)
            ) && n.belongs_to_build(&id)
        });

        self.perform().await?;

        match subscription.next_message(timeout).await? {
            GreybelNotification::BuildFinishedRpc(m) => Ok(BuildOutcome::Completion(m)),
            GreybelNotification::DisposedBuildRpc(m) => Ok(BuildOutcome::Disposal(m)),
            other => Err(ClientError::UnexpectedResponse(other.name())),
        }
    }

    pub async fn state(&self) -> Result<BuildStateClientRpc<'static>, ClientError> {
        let request = BuildStateServerRpc {
            build_id: self.build_id.clone().into(),
        };

        match self
            .request(request, |n| {
                matches!(n, GreybelNotification::BuildStateClientRpc(_))
            })
            .await?
        {
            GreybelNotification::BuildStateClientRpc(m) => Ok(m),
            other => Err(ClientError::UnexpectedResponse(other.name())),
        }
    }

    pub async fn dispose(&self) -> Result<DisposedBuildRpc<'static>, ClientError> {
        let request = DisposeBuildRpc {
            build_id: self.build_id.clone().into(),
        };

        match self
            .request(request, |n| {
                matches!(n, GreybelNotification::DisposedBuildRpc(_))
            })
            .await?
        {
            GreybelNotification::DisposedBuildRpc(m) => Ok(m),
            other => Err(ClientError::UnexpectedResponse(other.name())),
        }
    }
}
