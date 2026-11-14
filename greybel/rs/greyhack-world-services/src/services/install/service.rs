//! Service file installation and port registration (`PlayerUtils.InstallService`).

use greyhack_behaviour::{
    DOWNLOADS_PATH, DeviceKind, FIRST_VERSION, FileData, ServiceId, WEBSITE_PATH, is_device, path,
};
use greyhack_world_assets::{Script, content_file, ssh_decode, ssh_encode, website_default};

use crate::model::service_config::config_text;
use crate::{
    ComputerId, DEFAULT_DECODE_PATH, DEFAULT_ENCODE_PATH, Port, RepositoryConfig, Result,
    SshdConfig,
};

/// Methods of [`World`] for services install service.
impl Installs {
    pub async fn service_address(
        &self,
        computer: ComputerId,
        service: ServiceId,
    ) -> Result<String> {
        let local_ip = self
            .networks
            .topology
            .computer(computer)
            .await?
            .map(|computer| computer.local_ip)
            .unwrap_or_default();
        let port = self
            .computers
            .processes
            .ports(computer)
            .await?
            .into_iter()
            .find(|port| port.service == service)
            .map_or(service.default_port(), |port| port.number);

        Ok(format!("{local_ip}:{port}"))
    }

    pub async fn install_service(
        &self,
        computer: ComputerId,
        id: ServiceId,
        version: &str,
    ) -> Result<bool> {
        let domain = self.mail.mail_domain(computer).await?;

        if !self.computers.has_computer(computer).await? {
            return Ok(false);
        }

        self.configure(computer, id, &domain).await?;

        if !self.computers.processes.has_service(computer, id).await? {
            let library = self.files.library_row(id.library_kind(), version).await?;

            self.computers
                .insert_port(
                    computer,
                    &Port {
                        number: id.default_port(),
                        service: id,
                        version: version.to_owned(),
                        closed: false,
                        visible: true,
                    },
                    library,
                )
                .await?;
        }

        Ok(true)
    }

    pub async fn install_smartphone_adb(&self, computer: ComputerId) -> Result<bool> {
        if self
            .networks
            .topology
            .computer(computer)
            .await?
            .is_none_or(|computer| computer.kind != greyhack_behaviour::DeviceKind::Smartphone)
        {
            return Ok(false);
        }

        self.install_service(computer, ServiceId::Adb, FIRST_VERSION)
            .await
    }
}

impl Installs {
    pub async fn configure(&self, computer: ComputerId, id: ServiceId, domain: &str) -> Result<()> {
        let daemon = Script::find(id.daemon());
        let data = daemon.map_or_else(FileData::default, |script| {
            FileData::binary_from_source(script.source)
        });

        self.files
            .changes
            .write_file(computer, id.executable_path(), data)
            .await?;
        self.files
            .changes
            .write_file(computer, id.config_path(), FileData::text(""))
            .await?;

        match id {
            ServiceId::Ssh => {
                self.files
                    .changes
                    .write_file(
                        computer,
                        id.config_path(),
                        FileData::text(&config_text(&SshdConfig::default())),
                    )
                    .await?;
                self.files
                    .changes
                    .write_file(computer, DEFAULT_ENCODE_PATH, FileData::text(ssh_encode()))
                    .await?;
                self.files
                    .changes
                    .write_file(
                        computer,
                        DEFAULT_DECODE_PATH,
                        FileData::binary_from_source(ssh_decode()),
                    )
                    .await?;
            }
            ServiceId::Ftp => {
                self.files
                    .changes
                    .ensure_folder(computer, "/Public/Downloads")
                    .await?;
            }
            ServiceId::Http => {
                let kind = self
                    .networks
                    .topology
                    .computer(computer)
                    .await?
                    .map(|computer| computer.kind);
                let router = kind.is_some_and(is_device);

                // Router configuration files remain empty (`Computer.PreConfigServices`).
                if !router {
                    self.files
                        .changes
                        .write_file(
                            computer,
                            id.config_path(),
                            FileData::text(
                                r#"{
  "softwareDescription": {
    "myprogram": {
      "description": "Example description",
      "price": 0
    }
  }
}"#,
                            ),
                        )
                        .await?;
                }

                self.files
                    .changes
                    .ensure_folder(computer, DOWNLOADS_PATH)
                    .await?;

                // `Computer.ConfiguringServices`: website configuration for routers and cameras.
                let website = match kind {
                    _ if router => content_file("Router"),
                    Some(DeviceKind::Camera) => content_file("cctv"),
                    _ => website_default(),
                };

                // The website is only added where there is none (`FileSystem.Carpeta.GetSubArchivo`): a mission's
                // web file written before the service is installed in the world stays.
                if self.files.node_at(computer, WEBSITE_PATH).await?.is_none() {
                    self.files
                        .changes
                        .write_file(computer, WEBSITE_PATH, FileData::text(website))
                        .await?;
                }
            }
            ServiceId::Smtp => {
                self.files
                    .changes
                    .write_file(computer, id.config_path(), FileData::text(domain))
                    .await?;
            }
            ServiceId::Criminals | ServiceId::Employees | ServiceId::Students => {
                let viewer = id.record_viewer().expect("a SQL service has a viewer");
                let target = self.service_address(computer, id).await?;

                self.write_launcher(computer, &format!("/root/{viewer}"), &target)
                    .await?;
            }
            ServiceId::Rshell
            | ServiceId::BankAccount
            | ServiceId::Cam
            | ServiceId::SmartAppliance
            | ServiceId::Adb => {}
            ServiceId::Repository => {
                let packages = [
                    (
                        "helloworld",
                        "Prints the words hello world on the screen.",
                        "print(\"hello world!\")",
                    ),
                    (
                        "heythere",
                        "Prints the words hey there on the screen.",
                        "print(\"hey there!\")",
                    ),
                ];
                let config = RepositoryConfig {
                    repo_sources: packages
                        .iter()
                        .map(|(name, description, _)| {
                            ((*name).to_owned(), (*description).to_owned())
                        })
                        .collect(),
                    ..RepositoryConfig::default()
                };

                self.files
                    .changes
                    .write_file(
                        computer,
                        id.config_path(),
                        FileData::text(&config_text(&config)),
                    )
                    .await?;

                for (name, _, source) in packages {
                    self.files
                        .changes
                        .write_file(
                            computer,
                            &path::join(&config.path_repo, name),
                            FileData::binary_from_source(source),
                        )
                        .await?;
                }
            }
        }

        Ok(())
    }
}

/// A world service with only its declared dependencies.
pub struct Installs {
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_core::Dep<greyhack_world_core::FileSystems>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    pub(crate) mail: greyhack_world_core::Dep<greyhack_world_npcs::Mail>,
}

impl Installs {
    pub(crate) fn new(
        computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
        files: greyhack_world_core::Dep<greyhack_world_core::FileSystems>,
        networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
        mail: greyhack_world_core::Dep<greyhack_world_npcs::Mail>,
    ) -> Self {
        Self {
            computers,
            files,
            networks,
            mail,
        }
    }
}
