//! The rules of `Router.GenerateRandomContent` checked on computers of the game and of the simulation alike.

use std::collections::BTreeMap;

use greyhack_behaviour::{FileData, FileKind, PermissionSet};
use greyhack_world_assets::WorldItemTable;

/// A file or folder of a computer, as the game and the world both can list it.
pub struct Entry {
    pub path: String,
    pub folder: bool,
    pub owner: String,
    pub group: String,
    pub permissions: String,
    pub kind: FileKind,
    pub content: String,
    pub size: u64,
}

/// A computer of a generated network: whether it is a standard computer rather than a camera, appliance or phone, its
/// accounts and its entries.
pub struct Computer {
    pub standard: bool,
    pub users: Vec<String>,
    pub entries: Vec<Entry>,
}

/// The random files, images and chat logs found in the homes of `computers`.
pub fn random_files(computers: &[Computer]) -> usize {
    computers
        .iter()
        .map(|computer| homes(computer).values().map(Vec::len).sum::<usize>())
        .sum()
}

/// Whether `entry` can come from `GenerateRandomContent`. Mission chat logs and documents (`Router` mission
/// generation) and rumor chat logs (`NpcComputer.ConfigNpcRumors`) are also chat logs and documents of a home, but
/// their texts are not in the game's `WorldItems`, the list random content draws from.
fn is_random_content(entry: &Entry) -> bool {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&entry.content) else {
        return true;
    };
    let items = WorldItemTable::get();
    let id = match entry.kind {
        FileKind::ChatLog => json["chatID"].as_str(),
        FileKind::Pdf => json["textID"].as_str(),
        _ => return true,
    };

    id.is_none_or(|id| {
        items
            .texts
            .iter()
            .chain(&items.neurobox_texts)
            .any(|item| item.id == id)
    })
}

/// The files of each home that are of a type `GenerateRandomContent` adds, by account.
fn homes(computer: &Computer) -> BTreeMap<&str, Vec<&Entry>> {
    let mut homes: BTreeMap<&str, Vec<&Entry>> = BTreeMap::new();

    for user in &computer.users {
        let prefix = format!("/home/{user}/");

        for entry in &computer.entries {
            if !entry.folder
                && is_random_content(entry)
                && matches!(
                    entry.kind,
                    FileKind::ChatLog | FileKind::Image | FileKind::Pdf
                )
                && entry
                    .path
                    .strip_prefix(&prefix)
                    .is_some_and(|name| !name.contains('/'))
            {
                homes.entry(user).or_default().push(entry);
            }
        }
    }

    homes
}

/// Every way the random content of `computers` breaks what the game's code allows, one line each.
pub fn violations(computers: &[Computer]) -> Vec<String> {
    let items = WorldItemTable::get();
    let texts: Vec<&str> = items
        .texts
        .iter()
        .chain(&items.neurobox_texts)
        .map(|item| item.id.as_str())
        .collect();
    let mut found = Vec::new();

    for (index, computer) in computers.iter().enumerate() {
        let homes = homes(computer);

        if !computer.standard && !homes.is_empty() {
            found.push(format!("computer {index}: a device holds random files"));
        }

        if homes.len() > 1 {
            found.push(format!("computer {index}: random files of several users"));
        }

        for (user, files) in homes {
            let images = files
                .iter()
                .filter(|file| file.kind == FileKind::Image)
                .count();
            let others = files.len() - images;
            let mut names: Vec<&str> = files
                .iter()
                .map(|file| file.path.rsplit('/').next().unwrap_or_default())
                .collect();

            names.sort_unstable();
            names.dedup();

            if images > 2 || others > 2 {
                found.push(format!("{user}: {others} files and {images} images"));
            }

            if names.len() != files.len() {
                found.push(format!("{user}: two files of a name"));
            }

            for file in files {
                let name = file.path.rsplit('/').next().unwrap_or_default();

                if file.owner != user || file.group != user {
                    found.push(format!(
                        "{}: owned by {}:{}",
                        file.path, file.owner, file.group
                    ));
                }

                if file.permissions != PermissionSet::default_for(false).as_str() {
                    found.push(format!("{}: permissions {}", file.path, file.permissions));
                }

                if file.size != FileData::binary_size(name, false) {
                    found.push(format!("{}: size {}", file.path, file.size));
                }

                match file.kind {
                    FileKind::ChatLog => chat_log(file, name, user, &texts, &mut found),
                    FileKind::Pdf => document(file, name, &mut found),
                    _ => {
                        if !file.content.is_empty()
                            || !items.images.iter().any(|image| image.name == name)
                        {
                            found.push(format!("{}: not an image of the game", file.path));
                        }
                    }
                }
            }
        }
    }

    found
}

/// Checks a chat log is the JSON of `HelperConversation` between `user` and a first name.
fn chat_log(file: &Entry, name: &str, user: &str, texts: &[&str], found: &mut Vec<String>) {
    let digits = name
        .strip_prefix("chat")
        .and_then(|name| name.strip_suffix(".log"))
        .unwrap_or_default();

    if digits.is_empty() || digits.len() > 3 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        found.push(format!("{}: not a numbered chat log", file.path));
    }

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&file.content) else {
        found.push(format!("{}: not JSON", file.path));

        return;
    };
    let id = json["chatID"].as_str().unwrap_or_default();
    let receive = json["users"]["RECEIVE"].as_str().unwrap_or_default();
    let rebuilt = format!(
        r#"{{"chatID":{},"users":{{"SEND":{},"RECEIVE":{}}},"idxPublicIPs":{{}},"idxLocalIPs":{{}},"idxNpcs":{{}},"runtimeTags":{{}},"zeroDayNetworkIP":""}}"#,
        serde_json::to_string(id).unwrap_or_default(),
        serde_json::to_string(user).unwrap_or_default(),
        serde_json::to_string(receive).unwrap_or_default(),
    );

    if rebuilt != file.content {
        found.push(format!("{}: content differs from {rebuilt}", file.path));
    }

    if !texts.contains(&id) || !id.contains("CONV") || id.contains("CASUAL_M") {
        found.push(format!("{}: conversation {id}", file.path));
    }

    if !receive.chars().next().is_some_and(char::is_uppercase) {
        found.push(format!("{}: name {receive}", file.path));
    }
}

/// Checks a document is the JSON of `HelperPDF` of a text of the game, named after it.
fn document(file: &Entry, name: &str, found: &mut Vec<String>) {
    let items = WorldItemTable::get();
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&file.content) else {
        found.push(format!("{}: not JSON", file.path));

        return;
    };
    let id = json["textID"].as_str().unwrap_or_default();
    let rebuilt = format!(
        r#"{{"textID":{},"imgID":"","idxPublicIPs":{{}},"idxLocalIPs":{{}},"idxNpcs":{{}},"runtimeTags":{{}}}}"#,
        serde_json::to_string(id).unwrap_or_default(),
    );

    if rebuilt != file.content {
        found.push(format!("{}: content differs from {rebuilt}", file.path));
    }

    if !items
        .texts
        .iter()
        .chain(&items.neurobox_texts)
        .any(|item| item.id == id && item.name == name && id.contains("FILE"))
    {
        found.push(format!("{}: document {id} is not named {name}", file.path));
    }
}
