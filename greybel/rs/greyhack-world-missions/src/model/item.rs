use std::collections::BTreeMap;

use super::{FileItem, FirewallItem};

/// Runtime tags of an item (`MissionItem.runtimeTags`), lines in its texts.
pub type TagMap = BTreeMap<String, String>;

/// What an item is (`MissionConfig.TypeItem` and the class of each).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
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
    Firewall(FirewallItem),
}

/// `MissionConfig.MissionItem`: what a computer of a mission is given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionItem {
    /// The text the item shows (`MissionItem.ID`), or for a service the text of its website.
    pub id: String,
    pub is_evidence: bool,
    pub runtime_tags: TagMap,
    pub kind: ItemKind,
}

impl MissionItem {
    pub fn new(id: &str, kind: ItemKind) -> Self {
        Self {
            id: id.to_owned(),
            is_evidence: false,
            runtime_tags: TagMap::new(),
            kind,
        }
    }
}
