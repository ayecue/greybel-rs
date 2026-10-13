//! Service file installation and port registration (`PlayerUtils.InstallService`).

use greyhack_behaviour::{
    DOWNLOADS_PATH, DeviceKind, FIRST_VERSION, FileData, ServiceId, WEBSITE_PATH, is_device, path,
};
use greyhack_world_assets::{Script, content_file, ssh_decode, ssh_encode, website_default};

use crate::model::network::service_config::config_text;
use crate::{
    ComputerId, DEFAULT_DECODE_PATH, DEFAULT_ENCODE_PATH, Port, RepositoryConfig, Result,
    SshdConfig, World,
};

impl World {
    /// Returns `local_ip:port` for `computer` `service`, using its default port when none is open.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub(crate) async fn service_address(
        &self,
        computer: ComputerId,
        service: ServiceId,
    ) -> Result<String> {
        let local_ip = self
            .computer(computer)
            .await?
            .map(|computer| computer.local_ip)
            .unwrap_or_default();
        let port = self
            .ports(computer)
            .await?
            .into_iter()
            .find(|port| port.service == service)
            .map_or(service.default_port(), |port| port.number);

        Ok(format!("{local_ip}:{port}"))
    }

    /// Installs a service and opens its port without spawning another daemon.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn install_service(
        &mut self,
        computer: ComputerId,
        id: ServiceId,
        version: &str,
    ) -> Result<bool> {
        let domain = self.mail_domain(computer).await?;

        if !self.db.has_computer(computer).await? {
            return Ok(false);
        }

        self.configure(computer, id, &domain).await?;

        if !self.has_service(computer, id).await? {
            let library = self.db.library_row(id.library_kind(), version).await?;

            self.db
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

    async fn configure(&mut self, computer: ComputerId, id: ServiceId, domain: &str) -> Result<()> {
        let daemon = Script::find(id.daemon());
        let data = daemon.map_or_else(FileData::default, |script| {
            FileData::binary_from_source(script.source)
        });

        self.write_file(computer, id.executable_path(), data)
            .await?;
        self.write_file(computer, id.config_path(), FileData::text(""))
            .await?;

        match id {
            ServiceId::Ssh => {
                self.write_file(
                    computer,
                    id.config_path(),
                    FileData::text(&config_text(&SshdConfig::default())),
                )
                .await?;
                self.write_file(computer, DEFAULT_ENCODE_PATH, FileData::text(ssh_encode()))
                    .await?;
                self.write_file(
                    computer,
                    DEFAULT_DECODE_PATH,
                    FileData::binary_from_source(ssh_decode()),
                )
                .await?;
            }
            ServiceId::Ftp => {
                self.ensure_folder(computer, "/Public/Downloads").await?;
            }
            ServiceId::Http => {
                let kind = self.computer(computer).await?.map(|computer| computer.kind);
                let router = kind.is_some_and(is_device);

                // Router configuration files remain empty (`Computer.PreConfigServices`).
                if !router {
                    self.write_file(
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

                self.ensure_folder(computer, DOWNLOADS_PATH).await?;

                // `Computer.ConfiguringServices`: website configuration for routers and cameras.
                let website = match kind {
                    _ if router => content_file("Router"),
                    Some(DeviceKind::Camera) => content_file("cctv"),
                    _ => website_default(),
                };

                self.write_file(computer, WEBSITE_PATH, FileData::text(website))
                    .await?;
            }
            ServiceId::Smtp => {
                self.write_file(computer, id.config_path(), FileData::text(domain))
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

                self.write_file(
                    computer,
                    id.config_path(),
                    FileData::text(&config_text(&config)),
                )
                .await?;

                for (name, _, source) in packages {
                    self.write_file(
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

    /// Installs ADB on a smartphone after social-engineering mail acceptance
    /// (`Persona.ProcesaIngenieraSocial`). Returns false for other device kinds.
    pub async fn install_smartphone_adb(&mut self, computer: ComputerId) -> Result<bool> {
        if self
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
