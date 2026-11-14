//! The JSON of the authored mission lists, as the game's classes read it.

use greyhack_behaviour::{MissionKind, NetworkKind};
use serde::{Deserialize, Deserializer};

use crate::config::CompanionFileSpec;
use crate::model::{
    ComputerMission, FileItem, FirewallAction, FirewallItem, FirewallMatch, ItemKind, Mission,
    MissionItem, PreGenNpc, TagMap,
};

fn text<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

fn minus_one() -> i32 {
    -1
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RawMission {
    computers: Vec<RawComputer>,
    pre_gen_npcs: Vec<RawNpc>,
    #[serde(default = "minus_one")]
    closure_network_index: i32,
    #[serde(default = "minus_one")]
    closure_local_index: i32,
    #[serde(rename = "starterTextID", deserialize_with = "text")]
    starter_text_id: String,
    #[serde(default = "minus_one")]
    procedural_global_npc_index: i32,
    #[serde(default = "minus_one")]
    procedural_level: i32,
    #[serde(default = "minus_one")]
    reward_tier: i32,
}

impl Default for RawMission {
    fn default() -> Self {
        Self {
            computers: Vec::new(),
            pre_gen_npcs: Vec::new(),
            closure_network_index: -1,
            closure_local_index: -1,
            starter_text_id: String::new(),
            procedural_global_npc_index: -1,
            procedural_level: -1,
            reward_tier: -1,
        }
    }
}

impl RawMission {
    pub fn into_mission(self, kind: MissionKind) -> Mission {
        Mission {
            kind,
            computers: self
                .computers
                .into_iter()
                .map(RawComputer::into_computer)
                .collect(),
            pre_gen_npcs: self
                .pre_gen_npcs
                .into_iter()
                .map(RawNpc::into_npc)
                .collect(),
            closure_network_index: self.closure_network_index,
            closure_local_index: self.closure_local_index,
            starter_text_id: self.starter_text_id,
            procedural_global_npc_index: self.procedural_global_npc_index,
            procedural_level: self.procedural_level,
            reward_tier: self.reward_tier,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawNpc {
    is_victim: bool,
    is_guilty: bool,
    is_forced_pass: bool,
}

impl RawNpc {
    fn into_npc(self) -> PreGenNpc {
        PreGenNpc {
            is_victim: self.is_victim,
            is_guilty: self.is_guilty,
            is_forced_pass: self.is_forced_pass,
            ..PreGenNpc::default()
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawComputer {
    #[serde(default)]
    mission_part: Vec<RawItem>,
    #[serde(default)]
    tipo_red: i64,
    #[serde(default = "yes")]
    is_random_tipo_red: bool,
    #[serde(default)]
    computer_index_net: i32,
    #[serde(default = "minus_one")]
    clone_npc_pc_idx: i32,
}

impl RawComputer {
    fn into_computer(self) -> ComputerMission {
        ComputerMission {
            network_kind: (!self.is_random_tipo_red)
                .then(|| NetworkKind::from_code(self.tipo_red))
                .flatten(),
            computer_index_net: self.computer_index_net,
            clone_npc_pc_idx: self.clone_npc_pc_idx,
            procedural_archetype: None,
            items: self
                .mission_part
                .into_iter()
                .filter_map(RawItem::into_item)
                .collect(),
        }
    }
}

/// Every field of every class of item (`MissionConfig.MissionItem` and its subclasses), read by `typeItem`
/// (`MissionItemConverter`).
#[derive(Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RawItem {
    #[serde(rename = "ID", deserialize_with = "text")]
    id: String,
    type_item: i32,
    is_evidence: bool,
    force_npc_receive: bool,
    #[serde(default = "minus_one")]
    index_npc: i32,
    #[serde(default = "yes")]
    random_path: bool,
    #[serde(deserialize_with = "text")]
    path: String,
    use_network_npc_sender: bool,
    #[serde(rename = "titleID", deserialize_with = "text")]
    title_id: String,
    action: i32,
    port: i32,
    to_computer_idx: i32,
    is_receiving_conn: bool,
    servicio: u8,
    allow_duplicates: bool,
    status_port: i32,
    #[serde(rename = "statusSourceIP")]
    status_source_ip: i32,
    #[serde(rename = "pcIndexSourceIP", default = "minus_one")]
    pc_index_source_ip: i32,
    #[serde(rename = "statusDestIP")]
    status_dest_ip: i32,
    #[serde(rename = "pcIndexDestIP", default = "minus_one")]
    pc_index_dest_ip: i32,
    row_shuffle_seed: i32,
    #[serde(deserialize_with = "text")]
    file_name: String,
    is_pdf: bool,
    is_binary: bool,
    is_script: bool,
    allow_import: bool,
    companion_files: Vec<CompanionFileSpec>,
    is_image: bool,
    #[serde(deserialize_with = "text")]
    symlink_target: String,
    #[serde(deserialize_with = "text")]
    owner: String,
    #[serde(deserialize_with = "text")]
    permissions: String,
    #[serde(deserialize_with = "text")]
    directory_permissions: String,
    encrypt_with_owner_password: bool,
    #[serde(deserialize_with = "text")]
    encryption_password: String,
    is_web: bool,
    #[serde(deserialize_with = "text")]
    raw_content: String,
}

impl Default for RawItem {
    fn default() -> Self {
        Self {
            id: String::new(),
            type_item: -1,
            is_evidence: false,
            force_npc_receive: false,
            index_npc: -1,
            random_path: true,
            path: String::new(),
            use_network_npc_sender: false,
            title_id: String::new(),
            action: 0,
            port: 0,
            to_computer_idx: 0,
            is_receiving_conn: false,
            servicio: 0,
            allow_duplicates: false,
            status_port: 0,
            status_source_ip: 0,
            pc_index_source_ip: -1,
            status_dest_ip: 0,
            pc_index_dest_ip: -1,
            row_shuffle_seed: 0,
            file_name: String::new(),
            is_pdf: false,
            is_binary: false,
            is_script: false,
            allow_import: false,
            companion_files: Vec::new(),
            is_image: false,
            symlink_target: String::new(),
            owner: String::new(),
            permissions: String::new(),
            directory_permissions: String::new(),
            encrypt_with_owner_password: false,
            encryption_password: String::new(),
            is_web: false,
            raw_content: String::new(),
        }
    }
}

impl RawItem {
    /// The item, or `None` for a `typeItem` the converter reads as null.
    fn into_item(self) -> Option<MissionItem> {
        let mode = |status: i32| {
            if status == 0 {
                FirewallMatch::Any
            } else {
                FirewallMatch::Single
            }
        };
        let kind = match self.type_item {
            0 => ItemKind::ChatLog {
                force_npc_receive: self.force_npc_receive,
                index_npc: self.index_npc,
                random_path: self.random_path,
                path: self.path,
            },
            1 => ItemKind::Mails {
                force_npc_receive: self.force_npc_receive,
                index_npc: self.index_npc,
                use_network_npc_sender: self.use_network_npc_sender,
                title_id: self.title_id,
            },
            2 => ItemKind::SystemLog {
                action: u8::try_from(self.action).ok()?,
                port: self.port,
                to_computer_idx: self.to_computer_idx,
                is_receiving_conn: self.is_receiving_conn,
            },
            3 => ItemKind::TextFile(Box::new(FileItem {
                row_shuffle_seed: self.row_shuffle_seed,
                file_name: self.file_name,
                random_path: self.random_path,
                is_pdf: self.is_pdf,
                is_binary: self.is_binary,
                is_script: self.is_script,
                allow_import: self.allow_import,
                companion_files: self.companion_files,
                is_image: self.is_image,
                path: self.path,
                symlink_target: self.symlink_target,
                owner: self.owner,
                permissions: self.permissions,
                directory_permissions: self.directory_permissions,
                encrypt_with_owner_password: self.encrypt_with_owner_password,
                encryption_password: self.encryption_password,
                is_web: self.is_web,
                raw_content: self.raw_content,
            })),
            4 => ItemKind::Service {
                service: self.servicio,
                allow_duplicates: self.allow_duplicates,
            },
            5 => ItemKind::Firewall(FirewallItem {
                action: if self.action == 0 {
                    FirewallAction::Allow
                } else {
                    FirewallAction::Deny
                },
                status_port: mode(self.status_port),
                port: self.port,
                status_source_ip: mode(self.status_source_ip),
                pc_index_source_ip: self.pc_index_source_ip,
                status_dest_ip: mode(self.status_dest_ip),
                pc_index_dest_ip: self.pc_index_dest_ip,
            }),
            _ => return None,
        };

        Some(MissionItem {
            id: self.id,
            is_evidence: self.is_evidence,
            runtime_tags: TagMap::new(),
            kind,
        })
    }
}
