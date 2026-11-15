//! Helpers of the procedural mission parity tests.

use greyhack_behaviour::{MissionKind, ServiceId};
use greyhack_test_support_client::TestClient;
use greyhack_world::World;
use greyhack_world_assets::{FileSystemLayout, NameList, content_file};
use serde_json::Value;

/// Makes Greybel draw names, surnames and passwords from the game's own lists. The bundle keeps them in a shuffled
/// order, so the seeded draws of `WordGenerator.GetNombre` and `MarkovNameGenerator.NextName` (and the draws after them
/// in the same `Random`, which depend on the length of the word drawn) only match when both sides read the same
/// texts. Call it before anything in the test process uses a name.
pub async fn use_game_word_lists(client: &TestClient) {
    let reply = client
        .query_word_lists()
        .await
        .expect("the game's name lists");
    let texts = reply.lists.iter().map(|list| {
        (
            NameList::of_resource(&list.name).expect("a name list of the game"),
            list.text.to_string(),
        )
    });

    assert!(
        NameList::replace_texts(texts),
        "a name list was read before the game's lists were set"
    );
}

/// Replaces every address of a LAN range in `text` with `<lan>`: the local addresses of the mission computers are
/// chosen from each network's own generator.
pub fn without_lan_addresses(text: &str) -> String {
    let mut result = String::new();
    let mut rest = text;

    while !rest.is_empty() {
        let candidate = ["10.0.", "172.16.", "192.168."]
            .iter()
            .filter_map(|prefix| rest.find(prefix))
            .min();

        let Some(at) = candidate else {
            result.push_str(rest);
            break;
        };

        result.push_str(&rest[..at]);

        let tail = &rest[at..];
        let length = tail
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(tail.len());

        result.push_str("<lan>");
        rest = &tail[length..];
    }

    result
}

/// A file item of a mission (`MissionConfig.FileItem`): where `Router.PostGenMission` puts it.
pub struct Wanted {
    name: String,
    folder: String,
    random_path: bool,
}

impl Wanted {
    /// Whether `path` is where the item is put: in the folder of the item's path, or under the home of the account the
    /// item is given to when its path is random (`"/home/" + user + path`).
    fn matches(&self, path: &str) -> bool {
        let folder = self.folder.trim_end_matches('/');
        let full = format!("{folder}/{}", self.name);

        if !self.random_path {
            return path == full;
        }

        path.strip_prefix("/home/")
            .and_then(|rest| rest.split_once('/'))
            .is_some_and(|(_, rest)| format!("/{rest}") == full)
    }
}

/// A file of a computer as the test compares it.
pub struct Observed {
    pub path: String,
    pub owner: String,
    pub permissions: String,
    pub content: String,
    pub binary: bool,
}

/// The file items of the mission's network `index`, from the mission JSON.
pub fn mission_files(mission: &Value, index: usize) -> Vec<Wanted> {
    mission["parts"]
        .get(index)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|computer| computer["items"].as_array().into_iter().flatten())
        .filter(|item| item["type"] == 3)
        .map(|item| Wanted {
            name: item["fileName"].as_str().unwrap_or_default().to_owned(),
            folder: item["path"].as_str().unwrap_or_default().to_owned(),
            random_path: item["randomPath"] == true,
        })
        .collect()
}

/// The texts of the web items of the mission's network `index` (`Router.PostGenMission` reads them from
/// `XmlGlobal.CacheText.textFiles` by the item's id).
pub fn mission_websites(mission: &Value, index: usize) -> Vec<String> {
    mission["parts"]
        .get(index)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|computer| computer["items"].as_array().into_iter().flatten())
        .filter(|item| item["type"] == 3 && item["isWeb"] == true)
        .map(|item| content_file(item["id"].as_str().unwrap_or_default()).to_owned())
        .collect()
}

/// Whether `file` is the file of the base layout at its path, as every computer starts with it
/// (`XmlGlobal.GenerarFileSystem`): a root-owned file with the layout's permissions and kind. A mission puts its file
/// over such a file (a `sources.txt` in `/etc/apt`, a script `ls` in `/bin`), and only the mission's is compared.
fn is_untouched_layout_file(file: &Observed) -> bool {
    let mut folder = None;
    let mut parts = file.path.trim_start_matches('/').split('/').peekable();
    let layout = FileSystemLayout::base();

    while let Some(part) = parts.next() {
        if parts.peek().is_some() {
            folder = match folder {
                None => layout.folder(part),
                Some(parent) => parent.folder(part),
            };

            if folder.is_none() {
                return false;
            }
        } else {
            return folder
                .and_then(|folder| folder.file(part))
                .is_some_and(|entry| {
                    file.owner == "root"
                        && file.permissions == entry.permissions
                        && file.binary == entry.binary
                });
        }
    }

    false
}

/// The files at the places of `wanted`, with their names, permissions and normalized texts, sorted. Chat logs and
/// documents are JSON that names their text, so their content is compared as it is. Every computer with the http
/// service has a `website.html` of its own (`LanPcInfo.website`, drawn from `XmlGlobal.DynamicWebContent.GetRandomContent` with the
/// network's random generator), so only those whose text is one of `websites`, the texts of the mission's web items,
/// are compared. The files are matched by where the mission puts them and not by name, because every computer has the
/// files of the base layout and the network sizes differ between the game and the simulation.
pub fn mission_found(
    files: impl Iterator<Item = Observed>,
    wanted: &[Wanted],
    websites: &[String],
) -> Vec<(String, String, String)> {
    let mut found: Vec<(String, String, String)> = files
        .filter(|file| {
            let name = file.path.rsplit('/').next().unwrap_or_default();

            wanted.iter().any(|item| item.matches(&file.path))
                && !is_untouched_layout_file(file)
                && (name != "website.html" || websites.contains(&file.content))
        })
        .map(|file| {
            (
                file.path.rsplit('/').next().unwrap_or_default().to_owned(),
                file.permissions,
                if file.binary {
                    String::new()
                } else {
                    without_lan_addresses(&file.content)
                },
            )
        })
        .collect();

    found.sort();

    found
}

pub async fn simulated_files(
    world: &World,
    computer: greyhack_world_core::ComputerId,
) -> Vec<Observed> {
    let mut found = Vec::new();
    let mut folders = vec![
        world
            .modules
            .core
            .files
            .lookup
            .root(computer)
            .await
            .unwrap()
            .unwrap(),
    ];

    while let Some(folder) = folders.pop() {
        for child in world
            .modules
            .core
            .files
            .lookup
            .children(&folder)
            .await
            .unwrap()
        {
            if child.is_folder() {
                folders.push(child);
            } else if let Some(data) = child.data() {
                found.push(Observed {
                    path: child.path.clone(),
                    owner: child.owner.clone(),
                    permissions: child.permissions.to_string(),
                    content: data.content.clone(),
                    binary: data.is_binary(),
                });
            }
        }
    }

    found
}

pub const KINDS: [MissionKind; 7] = [
    MissionKind::Secret,
    MissionKind::Grey,
    MissionKind::Black,
    MissionKind::White,
    MissionKind::ZeroDay,
    MissionKind::Procedural,
    MissionKind::PoliceProcedural,
];

/// The missions whose networks are generated: the first of each kind, and the hidden ones that set a network kind,
/// serve a website, or add firewall rules.
pub const GENERATED: [(MissionKind, u32); 11] = [
    (MissionKind::Secret, 0),
    (MissionKind::Secret, 2),
    (MissionKind::Secret, 4),
    (MissionKind::Secret, 9),
    (MissionKind::Grey, 0),
    (MissionKind::Grey, 2),
    (MissionKind::Black, 0),
    (MissionKind::White, 0),
    (MissionKind::ZeroDay, 0),
    (MissionKind::Procedural, 0),
    (MissionKind::PoliceProcedural, 0),
];

/// The number of rules the mission gives network `index`: the firewall items collected so far are added after each
/// computer (`NetworkLan.ConfigureChainMission`).
pub fn mission_rule_count(mission: &Value, index: usize) -> usize {
    let mut pending = 0;
    let mut total = 0;

    for computer in mission["parts"][index].as_array().into_iter().flatten() {
        pending += computer["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|item| item["type"] == 5)
            .count();
        total += pending;
    }

    if index == 0 && !chain_at_first(mission) {
        0
    } else {
        total
    }
}

/// Whether the first network of the mission is built by `ConfigureChainMission` (a procedural or hidden mission).
fn chain_at_first(mission: &Value) -> bool {
    matches!(
        mission["missionType"].as_str(),
        Some("HIDDEN" | "PROCEDURAL" | "POLICE_PROCEDURAL")
    )
}

pub fn zero_day_services(mission: &Value, index: usize) -> Vec<ServiceId> {
    mission["parts"][index]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|computer| computer["items"].as_array().into_iter().flatten())
        .filter(|item| item["type"] == 4)
        .filter_map(|item| ServiceId::from_code(item["service"].as_i64()?))
        .collect()
}
