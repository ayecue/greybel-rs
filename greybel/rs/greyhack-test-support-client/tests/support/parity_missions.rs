//! Helpers of the procedural mission parity tests.

use greyhack_test_support_client::TestClient;
use greyhack_world::World;
use greyhack_world_assets::NameList;
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

/// The names, permissions and normalized contents of the mission's file items of network `index`, from the mission JSON.
pub fn mission_files(mission: &Value, index: usize) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let Some(part) = mission["parts"].get(index).and_then(Value::as_array) else {
        return files;
    };

    for computer in part {
        for item in computer["items"].as_array().into_iter().flatten() {
            if item["type"] == 3 {
                files.push((
                    item["fileName"].as_str().unwrap_or_default().to_owned(),
                    item["permissions"].as_str().unwrap_or_default().to_owned(),
                ));
            }
        }
    }

    files.sort();

    files
}

/// The files with a name of `wanted` with their permissions and normalized texts, sorted. Chat logs and documents
/// are JSON that names their text, so their content is compared as it is.
pub fn names_and_permissions(
    files: impl Iterator<Item = (String, String, String, bool)>,
    wanted: &[(String, String)],
) -> Vec<(String, String, String)> {
    let mut found: Vec<(String, String, String)> = files
        .filter(|(path, ..)| {
            let name = path.rsplit('/').next().unwrap_or_default();

            wanted.iter().any(|(wanted, _)| wanted == name)
        })
        .map(|(path, permissions, content, binary)| {
            (
                path.rsplit('/').next().unwrap_or_default().to_owned(),
                permissions,
                if binary {
                    String::new()
                } else {
                    without_lan_addresses(&content)
                },
            )
        })
        .collect();

    found.sort();

    found
}

pub async fn simulated_files(
    world: &World,
    computer: greyhack_world::ComputerId,
) -> Vec<(String, String, String, bool)> {
    let mut found = Vec::new();
    let mut folders = vec![world.root(computer).await.unwrap().unwrap()];

    while let Some(folder) = folders.pop() {
        for child in world.children(&folder).await.unwrap() {
            if child.is_folder() {
                folders.push(child);
            } else if let Some(data) = child.data() {
                found.push((
                    child.path.clone(),
                    child.permissions.to_string(),
                    data.content.clone(),
                    data.is_binary(),
                ));
            }
        }
    }

    found
}
