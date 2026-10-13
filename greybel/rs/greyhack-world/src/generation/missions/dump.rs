//! A mission as JSON in the shape the real-game test bridge reads the game's mission in.

use greyhack_behaviour::WorldSeed;
use serde_json::{Value, json};

use super::config::Source;
use super::model::{ComputerMission, ItemKind, MissionItem, PreGenNpc};
use super::placement::mission_of_group;

fn npc(npc: &PreGenNpc) -> Value {
    json!({
        "name": npc.name,
        "surname": npc.surname,
        "isForcedPass": npc.is_forced_pass,
        "forcedPass": npc.forced_pass,
    })
}

fn item(item: &MissionItem) -> Value {
    let mut json = json!({
        "id": item.id,
        "isEvidence": item.is_evidence,
        "tags": item.runtime_tags,
    });
    let object = json.as_object_mut().expect("an object");
    let (kind, fields) = match &item.kind {
        ItemKind::ChatLog {
            force_npc_receive,
            index_npc,
            random_path,
            path,
        } => (
            0,
            json!({
                "forceNpcReceive": force_npc_receive,
                "indexNpc": index_npc,
                "randomPath": random_path,
                "path": path,
            }),
        ),
        ItemKind::Mails {
            force_npc_receive,
            index_npc,
            use_network_npc_sender,
            title_id,
        } => (
            1,
            json!({
                "forceNpcReceive": force_npc_receive,
                "indexNpc": index_npc,
                "useNetworkNpcSender": use_network_npc_sender,
                "titleId": title_id,
            }),
        ),
        ItemKind::SystemLog {
            action,
            port,
            to_computer_idx,
            is_receiving_conn,
        } => (
            2,
            json!({
                "action": action,
                "port": port,
                "toComputerIdx": to_computer_idx,
                "isReceivingConn": is_receiving_conn,
            }),
        ),
        ItemKind::TextFile(file) => (
            3,
            json!({
                "rowShuffleSeed": file.row_shuffle_seed,
                "fileName": file.file_name,
                "randomPath": file.random_path,
                "isPdf": file.is_pdf,
                "isBinary": file.is_binary,
                "isScript": file.is_script,
                "allowImport": file.allow_import,
                "isImage": file.is_image,
                "path": file.path,
                "symlinkTarget": file.symlink_target,
                "owner": file.owner,
                "permissions": file.permissions,
                "directoryPermissions": file.directory_permissions,
                "encryptWithOwnerPassword": file.encrypt_with_owner_password,
                "encryptionPassword": file.encryption_password,
                "rawContent": file.raw_content,
                "companions": file.companion_files.iter().map(|companion| json!({
                    "fileName": companion.file_name,
                    "content": companion.content,
                    "textId": companion.text_id,
                    "permissions": companion.permissions,
                    "encryptionPassword": companion.encryption_password,
                })).collect::<Vec<_>>(),
            }),
        ),
        ItemKind::Service {
            service,
            allow_duplicates,
        } => (
            4,
            json!({ "service": service, "allowDuplicates": allow_duplicates }),
        ),
    };

    object.insert("type".to_owned(), json!(kind));

    if let Some(fields) = fields.as_object() {
        object.extend(fields.clone());
    }

    json
}

fn computer(computer: &ComputerMission) -> Value {
    json!({
        "computerIndexNet": computer.computer_index_net,
        "cloneNpcPcIdx": computer.clone_npc_pc_idx,
        "archetype": computer.procedural_archetype.map(|archetype| archetype as u8),
        "items": computer.items.iter().map(item).collect::<Vec<_>>(),
    })
}

/// The procedural mission the game builds for `group` (`MissionGen.GetProceduralMission`) as JSON: its common
/// fields, its NPCs and the computers of each part; `None` where the game cannot build one.
#[must_use]
pub fn procedural_mission_json(seed: &WorldSeed, group: &[String], police: bool) -> Option<Value> {
    let source = if police {
        Source::Police
    } else {
        Source::Hackshop
    };
    let (_, mission) = mission_of_group(seed, group, source)?;
    let parts = mission.split();
    let first = parts.first()?;

    Some(json!({
        "starterTextId": first.starter_text_id,
        "closureNetworkIndex": first.closure_network_index,
        "closureLocalIndex": first.closure_local_index,
        "proceduralGlobalNpcIndex": first.procedural_global_npc_index,
        "proceduralLevel": first.procedural_level,
        "npcs": first.pre_gen_npcs.iter().map(npc).collect::<Vec<_>>(),
        "parts": parts.iter().map(|part| part.computers.iter().map(computer).collect::<Vec<_>>()).collect::<Vec<_>>(),
    }))
}
