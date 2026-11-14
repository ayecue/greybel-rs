//! A mission as JSON in the shape the real-game test bridge reads the game's mission in.

use greyhack_behaviour::{MissionKind, WorldSeed};
use serde_json::{Value, json};

use super::group::group_mission;
use super::model::{
    ComputerMission, FirewallAction, FirewallItem, FirewallMatch, ItemKind, MissionItem, PreGenNpc,
};

fn npc(npc: &PreGenNpc) -> Value {
    json!({
        "name": npc.name,
        "surname": npc.surname,
        "isVictim": npc.is_victim,
        "isGuilty": npc.is_guilty,
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
                "isWeb": file.is_web,
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
        ItemKind::Firewall(firewall) => (5, firewall_fields(firewall)),
    };

    object.insert("type".to_owned(), json!(kind));

    if let Some(fields) = fields.as_object() {
        object.extend(fields.clone());
    }

    json
}

fn firewall_fields(firewall: &FirewallItem) -> Value {
    let status = |status: FirewallMatch| match status {
        FirewallMatch::Any => 0,
        FirewallMatch::Single => 1,
    };

    json!({
        "action": match firewall.action {
            FirewallAction::Allow => 0,
            FirewallAction::Deny => 1,
        },
        "statusPort": status(firewall.status_port),
        "port": firewall.port,
        "statusSourceIp": status(firewall.status_source_ip),
        "pcIndexSourceIp": firewall.pc_index_source_ip,
        "statusDestIp": status(firewall.status_dest_ip),
        "pcIndexDestIp": firewall.pc_index_dest_ip,
    })
}

fn computer(computer: &ComputerMission) -> Value {
    json!({
        "networkKind": computer.network_kind.map(|kind| kind.code()),
        "computerIndexNet": computer.computer_index_net,
        "cloneNpcPcIdx": computer.clone_npc_pc_idx,
        "archetype": computer.procedural_archetype.map(|archetype| archetype as u8),
        "items": computer.items.iter().map(item).collect::<Vec<_>>(),
    })
}

/// The mission the game builds for `group` (`MissionGen.GetMission`, `MissionGen.GetProceduralMission`) as JSON: its
/// common fields, its NPCs and the computers of each part; `None` where the game cannot build one.
#[must_use]
pub fn mission_json(
    seed: &WorldSeed,
    group: &[String],
    kind: MissionKind,
    index: u32,
) -> Option<Value> {
    let mission = group_mission(seed, group, kind, index, true)?;
    let first = mission.parts.first()?;

    Some(json!({
        "missionType": first.kind.name(),
        "starterTextId": first.starter_text_id,
        "closureNetworkIndex": first.closure_network_index,
        "closureLocalIndex": first.closure_local_index,
        "proceduralGlobalNpcIndex": first.procedural_global_npc_index,
        "proceduralLevel": first.procedural_level,
        "npcs": first.pre_gen_npcs.iter().map(npc).collect::<Vec<_>>(),
        "parts": mission.parts.iter().map(|part| part.computers.iter().map(computer).collect::<Vec<_>>()).collect::<Vec<_>>(),
    }))
}
