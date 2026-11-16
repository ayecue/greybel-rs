//! Deploys build outputs and imports through Grey Hack's message-hook plugin.

#[cfg(feature = "game")]
use std::time::Duration;

use greybel_bundler::BuildOutput;
#[cfg(feature = "game")]
use greybel_bundler::Deployment;
#[cfg(feature = "game")]
use greyhack_message_hook_client::{
    BuildOutcome, BuildState, Client, ClientError, ClientOptions, ScriptType,
};

use crate::protocol::BuildFailure;

#[cfg(feature = "game")]
#[derive(Debug, thiserror::Error)]
enum DeployFailure {
    #[error("Grey Hack's message hook doesn't respond on port {port}: {source}")]
    Connect {
        port: u16,
        #[source]
        source: ClientError,
    },
    #[error("Grey Hack build failed: {source}")]
    Build {
        #[source]
        source: ClientError,
    },
    #[error("Grey Hack build failed: {}", outcome.error_message())]
    Outcome { outcome: BuildOutcome },
}

/// Game deployment timeout: 120 seconds.
#[cfg(feature = "game")]
const DEPLOY_TIMEOUT: Duration = Duration::from_secs(120);

/// Deploys `output` through the message hook on `port`; returns `Unavailable` without the `game` feature.
pub(crate) async fn deploy(port: u16, output: &BuildOutput) -> Result<(), BuildFailure> {
    #[cfg(feature = "game")]
    return output
        .deploy(port)
        .await
        .map_err(|error| BuildFailure::Unreachable {
            message: error.to_string(),
        });

    #[cfg(not(feature = "game"))]
    {
        let _ = (port, output);

        Err(BuildFailure::Unavailable)
    }
}

/// Deploys a `BuildOutput` through Grey Hack's message hook.
#[cfg(feature = "game")]
trait GameDeployment {
    /// Creates a build through the message hook on `port`, uploads files and scripts, optionally compiles, and waits
    /// for completion.
    async fn deploy(&self, port: u16) -> Result<(), DeployFailure>;
}

#[cfg(feature = "game")]
impl GameDeployment for BuildOutput {
    async fn deploy(&self, port: u16) -> Result<(), DeployFailure> {
        let client = Client::connect(ClientOptions::with_port(port))
            .await
            .map_err(|source| DeployFailure::Connect { port, source })?;
        let deployment = Deployment::new(self);
        let deploy_result = async {
            let build = client
                .create_build(
                    &deployment.directory,
                    &deployment.resources,
                    deployment.auto_compile,
                )
                .await?;

            build
                .add_script(ScriptType::Pre, &deployment.pre_script)
                .await?;
            for file in &deployment.files {
                build.add_resource(&file.path, &file.code).await?;
            }

            if let Some(script) = &deployment.post_script {
                build.add_script(ScriptType::Post, script).await?;
            }

            let outcome = build.perform_and_wait(Some(DEPLOY_TIMEOUT)).await;
            let _ = build.dispose().await;

            outcome
        }
        .await;

        client.shutdown().await;

        match deploy_result {
            Ok(outcome) if outcome.state() == BuildState::Complete => Ok(()),
            Ok(outcome) => Err(DeployFailure::Outcome { outcome }),
            Err(source) => Err(DeployFailure::Build { source }),
        }
    }
}
