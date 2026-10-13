//! What a mission requests of the world (`MissionConfig.Mission`, `ComputerMission`, `MissionItem` and its kinds,
//! `PreGenNpc`).

use std::collections::BTreeMap;

use super::config::{Archetype, CompanionFileSpec};

/// Runtime tags of an item (`MissionItem.runtimeTags`), lines in its texts.
pub(crate) type TagMap = BTreeMap<String, String>;

/// `MissionConfig.PreGenNpc`: an NPC a mission needs on its computers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PreGenNpc {
    pub name: String,
    pub surname: String,
    pub is_forced_pass: bool,
    pub forced_pass: Option<String>,
}

impl PreGenNpc {
    /// `PreGenNpc.GetCompleteName`.
    pub(crate) fn complete_name(&self) -> String {
        format!("{} {}", self.name, self.surname)
            .trim_start()
            .to_owned()
    }
}

/// `MissionConfig.FileItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileItem {
    pub row_shuffle_seed: i32,
    pub file_name: String,
    pub random_path: bool,
    pub is_pdf: bool,
    pub is_binary: bool,
    pub is_script: bool,
    pub allow_import: bool,
    pub companion_files: Vec<CompanionFileSpec>,
    pub is_image: bool,
    pub path: String,
    pub symlink_target: String,
    pub owner: String,
    pub permissions: String,
    pub directory_permissions: String,
    pub encrypt_with_owner_password: bool,
    pub encryption_password: String,
    pub raw_content: String,
}

/// What an item is (`MissionConfig.TypeItem` and the class of each).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ItemKind {
    ChatLog {
        force_npc_receive: bool,
        index_npc: i32,
        random_path: bool,
        path: String,
    },
    Mails {
        force_npc_receive: bool,
        index_npc: i32,
        use_network_npc_sender: bool,
        title_id: String,
    },
    SystemLog {
        /// `LogSystem.StatusLog` number.
        action: u8,
        port: i32,
        to_computer_idx: i32,
        is_receiving_conn: bool,
    },
    TextFile(Box<FileItem>),
    Service {
        /// `ServicioID` number.
        service: u8,
        allow_duplicates: bool,
    },
}

/// `MissionConfig.MissionItem`: what a computer of a mission is given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissionItem {
    /// The text the item shows (`MissionItem.ID`), or for a service the text of its website.
    pub id: String,
    pub is_evidence: bool,
    pub runtime_tags: TagMap,
    pub kind: ItemKind,
}

impl MissionItem {
    pub(crate) fn new(id: &str, kind: ItemKind) -> Self {
        Self {
            id: id.to_owned(),
            is_evidence: false,
            runtime_tags: TagMap::new(),
            kind,
        }
    }
}

/// `MissionConfig.ComputerMission`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComputerMission {
    pub computer_index_net: i32,
    pub clone_npc_pc_idx: i32,
    pub procedural_archetype: Option<Archetype>,
    pub items: Vec<MissionItem>,
}

impl ComputerMission {
    pub(crate) fn new(computer_index_net: i32) -> Self {
        Self {
            computer_index_net,
            clone_npc_pc_idx: -1,
            procedural_archetype: None,
            items: Vec::new(),
        }
    }
}

/// `MissionConfig.Mission`, as far as procedural missions use it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mission {
    pub computers: Vec<ComputerMission>,
    pub pre_gen_npcs: Vec<PreGenNpc>,
    pub closure_network_index: i32,
    pub closure_local_index: i32,
    pub starter_text_id: String,
    pub procedural_global_npc_index: i32,
    pub procedural_level: i32,
}

impl Default for Mission {
    fn default() -> Self {
        Self {
            computers: Vec::new(),
            pre_gen_npcs: Vec::new(),
            closure_network_index: -1,
            closure_local_index: -1,
            starter_text_id: String::new(),
            procedural_global_npc_index: -1,
            procedural_level: -1,
        }
    }
}

impl Mission {
    /// `MissionGen.SplitMission`: the part of each network of the chain, whose computers are those with its network
    /// index.
    pub(crate) fn split(&self) -> Vec<Self> {
        let mut parts = Vec::new();
        let mut tail = self.computers.clone();
        let mut network = 0;

        while !tail.is_empty() {
            let mut part = Self {
                computers: Vec::new(),
                pre_gen_npcs: self.pre_gen_npcs.clone(),
                closure_network_index: self.closure_network_index,
                closure_local_index: self.closure_local_index,
                starter_text_id: self.starter_text_id.clone(),
                procedural_global_npc_index: self.procedural_global_npc_index,
                procedural_level: self.procedural_level,
            };
            let mut index = tail.len();

            // The game walks the computers backwards and reverses what it took.
            while index > 0 {
                index -= 1;

                if tail[index].computer_index_net == network {
                    part.computers.push(tail.remove(index));
                }
            }

            part.computers.reverse();

            if !part.computers.is_empty() {
                parts.push(part);
            }

            network += 1;
        }

        parts
    }
}
