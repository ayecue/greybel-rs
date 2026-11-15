//! `crypto.so` (`CryptoIntrinsics`): file encryption, monitor mode, Wi-Fi capture, and key recovery.

use greyhack_behaviour::{
    FileKind, Permission, Refusal, check_password, hash_password, required_acks,
};
use greyhack_world_core::{
    ComputerId, FileFailure, NetCard, Node, NodeKind, Result, ScriptFileFailure,
};

use super::capture::AckCount;
use crate::WifiCapture;

/// Why a `crypto.so` operation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoFailure {
    /// The file is encrypted already.
    HasEncryption,
    /// The file is not encrypted.
    NoEncryption,
    /// The password does not decrypt the file.
    IncorrectPassword,
    /// The device is not a Wi-Fi card.
    NotWifiCard,
    /// The card cannot enter monitor mode.
    NoMonitorSupport,
    /// No router has the BSSID and ESSID.
    UnknownRouter,
    /// No network card is in monitor mode.
    NoMonitorCard,
    /// The user cannot read the capture.
    UnreadableCapture,
    /// The file is not a capture.
    InvalidCapture,
    /// The capture is encrypted.
    EncryptedCapture,
    /// The capture content cannot be parsed.
    CorruptedCapture,
    /// The capture needs more ACKs.
    MoreAcks,
    /// The file system refuses the file operation.
    File(FileFailure),
    /// The password breaks an argument rule.
    Refusal(Refusal),
}

/// A Wi-Fi capture a script started (`CryptoIntrinsics.aireplay`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiCaptureRun {
    /// BSSID of the captured network.
    pub bssid: String,
    /// Packets and ACKs collected.
    pub count: AckCount,
    /// Why saving the capture file failed, if it did.
    pub save_failure: Option<ScriptFileFailure>,
}

/// Methods of [`World`] for intrusion crypto.
impl Crypto {
    pub async fn encrypt_file(
        &self,
        computer: ComputerId,
        user: &str,
        path: &str,
        password: &str,
    ) -> Result<Option<std::result::Result<(), CryptoFailure>>> {
        if let Err(refusal) = check_password(password) {
            return Ok(Some(Err(CryptoFailure::Refusal(refusal))));
        }

        let mut node = match self
            .permitted_file(computer, user, path, Permission::Write)
            .await?
        {
            None => return Ok(None),
            Some(Err(failure)) => return Ok(Some(Err(failure))),
            Some(Ok(node)) => node,
        };
        let data = node.data_mut().expect("a file node");

        if data.is_encrypted() {
            return Ok(Some(Err(CryptoFailure::HasEncryption)));
        }

        data.encrypt_password = hash_password(password);
        self.files.changes.save_node(&node).await?;

        Ok(Some(Ok(())))
    }

    pub async fn decrypt_file(
        &self,
        computer: ComputerId,
        user: &str,
        path: &str,
        password: &str,
    ) -> Result<Option<std::result::Result<(), CryptoFailure>>> {
        if let Err(refusal) = check_password(password) {
            return Ok(Some(Err(CryptoFailure::Refusal(refusal))));
        }

        let mut node = match self
            .permitted_file(computer, user, path, Permission::Write)
            .await?
        {
            None => return Ok(None),
            Some(Err(failure)) => return Ok(Some(Err(failure))),
            Some(Ok(node)) => node,
        };
        let data = node.data_mut().expect("a file node");

        if !data.is_encrypted() {
            return Ok(Some(Err(CryptoFailure::NoEncryption)));
        }

        if data.encrypt_password != hash_password(password) {
            return Ok(Some(Err(CryptoFailure::IncorrectPassword)));
        }

        data.encrypt_password.clear();
        self.files.changes.save_node(&node).await?;

        Ok(Some(Ok(())))
    }

    pub async fn is_file_encrypted(
        &self,
        computer: ComputerId,
        user: &str,
        path: &str,
    ) -> Result<Option<std::result::Result<bool, CryptoFailure>>> {
        Ok(
            match self
                .permitted_file(computer, user, path, Permission::Read)
                .await?
            {
                None => None,
                Some(Err(failure)) => Some(Err(failure)),
                Some(Ok(node)) => Some(Ok(node
                    .data()
                    .is_some_and(greyhack_behaviour::FileData::is_encrypted))),
            },
        )
    }

    pub async fn set_monitor_mode(
        &self,
        computer: ComputerId,
        device: &str,
        start: bool,
    ) -> Result<std::result::Result<bool, CryptoFailure>> {
        let cards = self.networks.net_cards.net_cards(computer).await?;
        let Some(index) = NetCard::position(&cards, device) else {
            return Ok(Ok(false));
        };
        let mut card = cards[index].clone();

        if !card.wifi {
            return Ok(Err(CryptoFailure::NotWifiCard));
        }

        if !card.monitor_support {
            return Ok(Err(CryptoFailure::NoMonitorSupport));
        }

        card.monitor_enabled = start;
        self.networks
            .net_cards
            .save_net_card(computer, index, &card)
            .await?;

        Ok(Ok(true))
    }

    pub async fn capture_wifi(
        &self,
        computer: ComputerId,
        user: &str,
        network: (&str, &str),
        max_acks: i32,
        folder: &str,
    ) -> Result<Option<std::result::Result<WifiCaptureRun, CryptoFailure>>> {
        let (bssid, essid) = network;
        // Search the computer's LAN first (`NetworkLan.FindRouterLan`), then the map.
        let own = self
            .networks
            .topology
            .computer(computer)
            .await?
            .map(|row| row.network);
        let lan_router = match own {
            Some(own) => {
                self.networks
                    .routers
                    .lan_router_by_wifi(own, bssid, essid)
                    .await?
            }
            None => None,
        };
        let target = match lan_router {
            Some(router) => Some((router.id, true)),
            None => self
                .networks
                .routers
                .router_by_wifi(bssid, essid)
                .await?
                .map(|router| (router.id, false)),
        };

        if own.is_none() {
            return Ok(None);
        }

        let Some((router, lan)) = (match target {
            Some((id, lan)) => self
                .networks
                .routers
                .router(id)
                .await?
                .map(|router| (router, lan)),
            None => None,
        }) else {
            return Ok(Some(Err(CryptoFailure::UnknownRouter)));
        };

        if !self
            .networks
            .net_cards
            .net_cards(computer)
            .await?
            .iter()
            .any(|card| card.monitor_enabled)
        {
            return Ok(Some(Err(CryptoFailure::NoMonitorCard)));
        }

        // LAN captures use full power; map captures derive power from router distance.
        let power = if lan { 100 } else { router.wifi_power() };
        // Respect a non-negative script limit. Without one, collect enough ACKs for `aircrack`; this
        // run cannot be stopped manually by the player.
        let limit = if max_acks >= 0 {
            max_acks
        } else {
            required_acks(power)
        };
        let count = self.collect_acks(power, limit);
        let capture = WifiCapture {
            power,
            num_acks: count.acks,
            bssid: bssid.to_owned(),
            essid: essid.to_owned(),
            public_ip: router.public_ip.clone(),
        };
        let content = capture.to_content();
        let path = format!("{folder}/file.cap");
        let mut save_failure = None;

        if let Some(mut node) = self.files.lookup.file(computer, &path, true).await? {
            if let Some(data) = node.data_mut() {
                *data = greyhack_behaviour::FileData::capture(&content);
                self.files.changes.save_node(&node).await?;
            }
        } else if let Err(failure) = self
            .files
            .scripts
            .create_file_in(
                computer,
                user,
                folder,
                "file.cap",
                greyhack_behaviour::FileData::capture(&content),
            )
            .await?
        {
            // The save failure `FileCapServerRpc` uses to close the connection.
            save_failure = Some(failure);
        }

        Ok(Some(Ok(WifiCaptureRun {
            bssid: capture.bssid,
            count,
            save_failure,
        })))
    }

    pub async fn recover_wifi_password(
        &self,
        computer: ComputerId,
        user: &str,
        path: &str,
    ) -> Result<Option<std::result::Result<String, CryptoFailure>>> {
        let capture = match self.capture_with_enough_acks(computer, user, path).await? {
            Ok(Some(capture)) => capture,
            Ok(None) => return Ok(None),
            Err(failure) => return Ok(Some(Err(failure))),
        };

        // Search map routers first, then LAN routers on the capture's source network.
        if let Some(router) = self
            .networks
            .routers
            .router_by_wifi(&capture.bssid, &capture.essid)
            .await?
        {
            return Ok(Some(Ok(router.wifi_password)));
        }

        let Some(origin) = self.networks.routers.router_at(&capture.public_ip).await? else {
            return Ok(None);
        };

        Ok(self
            .networks
            .routers
            .lan_router_by_wifi(origin.network, &capture.bssid, &capture.essid)
            .await?
            .map(|router| Ok(router.wifi_password)))
    }
}

impl Crypto {
    pub async fn permitted_file(
        &self,
        computer: ComputerId,
        user: &str,
        path: &str,
        permission: Permission,
    ) -> Result<Option<std::result::Result<Node, CryptoFailure>>> {
        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(None);
        }

        let Some(node) = self.files.lookup.file(computer, path, true).await? else {
            return Ok(Some(Err(CryptoFailure::File(FileFailure::MissingFile(
                path.to_owned(),
            )))));
        };

        if !self
            .files
            .permissions
            .user_has_permission(computer, user, &node, permission)
            .await?
        {
            return Ok(Some(Err(CryptoFailure::File(FileFailure::NoPermission))));
        }

        Ok(Some(Ok(node)))
    }

    pub async fn capture_with_enough_acks(
        &self,
        computer: ComputerId,
        user: &str,
        path: &str,
    ) -> Result<std::result::Result<Option<WifiCapture>, CryptoFailure>> {
        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(Ok(None));
        }

        let Some(node) = self.files.lookup.file(computer, path, true).await? else {
            return Ok(Ok(None));
        };

        if !self
            .files
            .permissions
            .user_has_permission(computer, user, &node, Permission::Read)
            .await?
        {
            return Ok(Err(CryptoFailure::UnreadableCapture));
        }

        let NodeKind::File(data) = &node.kind else {
            return Ok(Ok(None));
        };

        if !data.is_binary() || data.kind == FileKind::Program {
            return Ok(Err(CryptoFailure::InvalidCapture));
        }

        if data.is_encrypted() {
            return Ok(Err(CryptoFailure::EncryptedCapture));
        }

        let Some(capture) = WifiCapture::parse(&data.content) else {
            return Ok(Err(CryptoFailure::CorruptedCapture));
        };

        if !capture.has_enough_acks() {
            return Ok(Err(CryptoFailure::MoreAcks));
        }

        Ok(Ok(Some(capture)))
    }
}

/// A world service with only its declared dependencies.
pub struct Crypto {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
}

impl Crypto {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    ) -> Self {
        Self {
            world: world.clone(),
            files,
            networks,
        }
    }
}

impl greyhack_world_npcs::SmtpAccess for Crypto {
    fn access(
        &self,
        objects: &greyhack_world_programs::ObjectTableHandle<ComputerId>,
        object: greyhack_world_programs::protocol::ObjectId,
    ) -> greyhack_world_programs::RouteResult<
        greyhack_world_core::Access,
        greyhack_world_core::WorldError,
    > {
        crate::objects::ObjectTableExt::crypto(objects, object)
    }
}
