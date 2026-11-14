//! Editing the draft of a mission computer: the account a mission NPC replaces, and the files and
//! logs a mission adds.

use greyhack_behaviour::{
    FileKind, PermissionSet, ROOT_USER, SYSTEM_LOG_FOLDER, SYSTEM_LOG_NAME, SYSTEM_LOG_PATH,
    SystemLog,
};
use greyhack_world_missions::{Assignment, ComputerContent, MissionFile, PreGenNpc};

use crate::{ComputerDraft, FileSystemDraft, NodeDraft, NodeId};

/// The permissions of a folder the mission creates that the game gives every new folder.
pub(super) fn default_folder() -> PermissionSet {
    PermissionSet::default_for(true)
}

pub(super) fn system_log_data(log: &SystemLog) -> greyhack_behaviour::FileData {
    let mut data = greyhack_behaviour::FileData::text(&log.to_json());

    data.kind = FileKind::SystemLog;
    data.default_content = true;

    data
}

/// The account of `draft` the mission NPC `npc` takes over: the one already named like it, or one of the others the
/// mission's draw picks (`NetworkLan.UpdateNpcMissionNames`, `NetworkLan.GetMissionNpcReplacementIndex`).
pub(super) fn replaced_account(
    draft: &ComputerDraft,
    npc: &PreGenNpc,
    assignment: &Assignment,
) -> Option<String> {
    if draft.users.iter().any(|user| user.name == npc.surname) {
        return Some(npc.surname.clone());
    }

    let others: Vec<&str> = draft
        .users
        .iter()
        .filter(|user| user.name != ROOT_USER)
        .map(|user| user.name.as_str())
        .collect();

    if others.is_empty() {
        return None;
    }

    let at = assignment
        .replacement
        .map_or(0, |draw| (draw * others.len() as f64) as usize)
        .min(others.len() - 1);

    Some(others[at].to_owned())
}

/// `NetworkLan.UpdateNpcMissionNames`/`NpcInfo.UpdateNpcInfo` for the accounts: `old` becomes `new`, with `password`
/// when the mission forces one.
pub(super) fn rename_account(
    draft: &mut ComputerDraft,
    old: &str,
    new: &str,
    password: Option<&str>,
) {
    if old != new
        && let Some(user) = draft.users.iter_mut().find(|user| user.name == old)
    {
        new.clone_into(&mut user.name);
        user.groups = user
            .groups
            .iter()
            .map(|group| {
                if group == old {
                    new.to_owned()
                } else {
                    group.clone()
                }
            })
            .collect();

        if let Some(home) = draft.filesystem.folder(&format!("/home/{old}"), false) {
            draft.filesystem.node_mut(home).name = new.to_owned();
        }

        draft.filesystem.rename_owner(old, new);

        for active in &mut draft.active_users {
            if active == old {
                new.clone_into(active);
            }
        }
    }

    if let Some(password) = password
        && let Some(user) = draft.users.iter_mut().find(|user| user.name == new)
    {
        *user = crate::User {
            name: user.name.clone(),
            groups: user.groups.clone(),
            deletable: user.deletable,
            person: user.person,
            ..crate::User::new(new, password)
        };
    }

    draft.write_passwd(true);
}

/// Adds what the mission gives to the file system of the draft, in the game's order.
pub(super) fn add_content(fs: &mut FileSystemDraft, content: &ComputerContent) {
    if !content.log.is_empty() {
        let log = SystemLog {
            content_log: content.log.clone(),
        };
        let open_var = fs
            .folder(SYSTEM_LOG_FOLDER, false)
            .is_some_and(|var| fs.node(var).permissions.to_string() == "drwxrwxrwx");
        let owner = ROOT_USER.to_owned();
        let mut file = MissionFile {
            folder: SYSTEM_LOG_FOLDER.to_owned(),
            name: SYSTEM_LOG_NAME.to_owned(),
            data: system_log_data(&log),
            owner,
            permissions: open_var.then(|| "-rwxrwxrwx".to_owned()),
            symlink: String::new(),
            timestamp: None,
            folder_permissions: None,
            create_folder: false,
        };

        if fs.folder(SYSTEM_LOG_FOLDER, false).is_none() {
            file.create_folder = true;
        }

        put_file(fs, &content.owner, &file);
    }

    debug_assert!(SYSTEM_LOG_PATH.starts_with(SYSTEM_LOG_FOLDER));

    for file in &content.files {
        put_file(fs, &content.owner, file);
    }
}

/// Adds `file` as `Router.PostGenMission` does: the folder is made when the mission sets permissions on it (or
/// for objective files), otherwise a file whose folder is missing is not added.
pub(super) fn put_file(fs: &mut FileSystemDraft, creator: &str, file: &MissionFile) {
    let folder = if file.create_folder {
        Some(ensure_folder(fs, &file.folder, creator))
    } else {
        fs.folder(&file.folder, false)
    };
    let Some(folder) = folder else {
        return;
    };

    if let Some(permissions) = file
        .folder_permissions
        .as_deref()
        .and_then(PermissionSet::parse)
    {
        fs.node_mut(folder).permissions = permissions;
    }

    let node = mission_node(file);

    fs.add(folder, node);
}

/// The node `Router.PostGenMission` creates for `file`.
pub(super) fn mission_node(file: &MissionFile) -> NodeDraft {
    let mut node = NodeDraft::file(
        &file.name,
        &file.owner,
        PermissionSet::default_for(false),
        file.data.clone(),
    );

    if let Some(permissions) = file.permissions.as_deref().and_then(PermissionSet::parse) {
        node.permissions = permissions;
    }

    if !file.symlink.is_empty() {
        node.symlink.clone_from(&file.symlink);
    }

    if let Some(timestamp) = file.timestamp {
        node.timestamp = timestamp;
    }

    node
}

/// `FileSystem.GetLastFolder`: the folder at `path`, created with `owner` where missing.
pub(super) fn ensure_folder(fs: &mut FileSystemDraft, path: &str, owner: &str) -> NodeId {
    let mut current = fs.root();
    let mut full = String::new();

    for segment in path.split('/').filter(|segment| !segment.is_empty()) {
        full.push('/');
        full.push_str(segment);
        current = match fs.folder(&full, false) {
            Some(folder) => folder,
            None => fs.add(current, NodeDraft::folder(segment, owner, default_folder())),
        };
    }

    current
}

#[cfg(test)]
#[path = "tests/mission_drafts.rs"]
pub(crate) mod tests;
