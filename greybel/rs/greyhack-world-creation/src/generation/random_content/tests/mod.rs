#[path = "support/mod.rs"]
pub(crate) mod support;

use greyhack_behaviour::{FileData, FileKind};
use greyhack_world_assets::{GameTextTable, WorldItemTable};
use mono_random::Random;
use support::{SEEDS, all, draw, is_chat, users};

use super::RandomContent;

#[test]
fn should_draw_the_same_content_for_the_same_seed() {
    assert_eq!(draw(5, false), draw(5, false));
}

#[test]
fn should_put_every_file_into_the_home_of_one_account_other_than_root() {
    for content in all(false) {
        let mut owners: Vec<&str> = content
            .files
            .iter()
            .map(|file| file.user.as_str())
            .collect();

        owners.dedup();

        assert!(owners.len() <= 1);
        assert!(owners.iter().all(|owner| *owner != "root"));
    }
}

#[test]
fn should_use_root_when_it_is_the_only_account() {
    let root = vec!["root".to_owned()];
    let content = (0..SEEDS)
        .map(|seed| {
            RandomContent::draw(&mut Random::new(seed), &root, false, &|_, _| false, &|_| {
                true
            })
        })
        .find(|content| !content.files.is_empty())
        .expect("some seed draws a file");

    assert!(content.files.iter().all(|file| file.user == "root"));
}

#[test]
fn should_name_and_type_files_like_the_game() {
    let items = WorldItemTable::get();
    let mut kinds = std::collections::BTreeSet::new();

    for content in all(false) {
        for file in &content.files {
            match file.data.kind {
                FileKind::ChatLog => {
                    let number = file
                        .name
                        .strip_prefix("chat")
                        .and_then(|name| name.strip_suffix(".log"))
                        .and_then(|number| number.parse::<u32>().ok())
                        .expect("a numbered chat log");

                    assert!(number < 1000);
                }
                FileKind::Pdf => {
                    assert!(
                        items
                            .texts
                            .iter()
                            .any(|item| item.name == file.name && item.id.contains("FILE")),
                        "{}",
                        file.name
                    );
                }
                FileKind::Image => {
                    assert!(items.images.iter().any(|image| image.name == file.name));
                    assert!(file.data.content.is_empty());
                }
                other => panic!("{other:?}"),
            }

            assert_eq!(file.data.size, FileData::binary_size(&file.name, false));
            kinds.insert(file.data.kind.code());
        }
    }

    assert_eq!(
        kinds,
        [
            FileKind::ChatLog.code(),
            FileKind::Pdf.code(),
            FileKind::Image.code()
        ]
        .into()
    );
}

#[test]
fn should_write_chat_logs_and_documents_as_the_games_helpers_serialize_them() {
    let texts = GameTextTable::english();

    for content in all(false) {
        for file in content
            .files
            .iter()
            .filter(|file| file.data.kind != FileKind::Image)
        {
            let json: serde_json::Value = serde_json::from_str(&file.data.content).unwrap();

            if is_chat(file) {
                let id = json["chatID"].as_str().unwrap();

                assert!(id.contains("CONV") && !id.contains("CASUAL_M"), "{id}");
                assert!(texts.get(id).is_some());
                assert_eq!(json["users"]["SEND"], file.user.as_str());
                assert!(
                    json["users"]["RECEIVE"].as_str().is_some_and(|name| {
                        name.chars().next().is_some_and(char::is_uppercase)
                    })
                );
                assert!(
                    file.data
                        .content
                        .starts_with(&format!(r#"{{"chatID":"{id}","users":{{"SEND":"#)),
                    "{}",
                    file.data.content
                );
                assert!(file.data.content.ends_with(
                    r#""idxPublicIPs":{},"idxLocalIPs":{},"idxNpcs":{},"runtimeTags":{},"zeroDayNetworkIP":""}"#
                ));
            } else {
                let id = json["textID"].as_str().unwrap();

                assert!(id.contains("FILE"));
                assert_eq!(
                    file.data.content,
                    format!(
                        r#"{{"textID":"{id}","imgID":"","idxPublicIPs":{{}},"idxLocalIPs":{{}},"idxNpcs":{{}},"runtimeTags":{{}}}}"#
                    )
                );
            }
        }
    }
}

#[test]
fn should_add_one_or_two_files_in_half_of_the_computers() {
    let runs = all(false);
    let with_files = runs
        .iter()
        .filter(|content| {
            content
                .files
                .iter()
                .any(|file| file.data.kind != FileKind::Image)
        })
        .count();

    // Half draw the file branch, of which a few draw only mail-only conversations.
    assert!(with_files > SEEDS as usize * 35 / 100, "{with_files}");
    assert!(with_files < SEEDS as usize * 50 / 100, "{with_files}");

    for content in &runs {
        let files = content
            .files
            .iter()
            .filter(|file| file.data.kind != FileKind::Image)
            .count();
        let images = content
            .files
            .iter()
            .filter(|file| file.data.kind == FileKind::Image)
            .count();

        assert!(files <= 2 && images <= 2);
    }
}

#[test]
fn should_skip_files_whose_name_the_home_already_holds() {
    let all_exist = |seed: i32| {
        RandomContent::draw(
            &mut Random::new(seed),
            &users(),
            false,
            &|_, _| true,
            &|_| true,
        )
    };

    let mut skipped = 0;

    for seed in 0..SEEDS {
        let content = all_exist(seed);

        assert!(
            content
                .files
                .iter()
                .all(|file| file.data.kind == FileKind::Image)
        );
        skipped += usize::from(
            draw(seed, false)
                .files
                .iter()
                .any(|file| file.data.kind != FileKind::Image),
        );
        // The numbers drawn do not depend on the skipped files.
        assert_eq!(content.mails, draw(seed, false).mails);
    }

    assert!(skipped > 0);
}

#[test]
fn should_skip_a_file_named_like_one_drawn_before() {
    // With nine texts in a Neurobox network, two draws of the same document are common.
    for content in all(true) {
        let mut names: Vec<&str> = content
            .files
            .iter()
            .map(|file| file.name.as_str())
            .collect();
        let count = names.len();

        names.sort_unstable();
        names.dedup();

        assert_eq!(names.len(), count);
    }
}

#[test]
fn should_add_images_in_three_of_ten_computers() {
    let images = all(false)
        .iter()
        .filter(|content| {
            content
                .files
                .iter()
                .any(|file| file.data.kind == FileKind::Image)
        })
        .count();

    // 40% after the file branch, 20% otherwise.
    assert!(images > SEEDS as usize * 25 / 100, "{images}");
    assert!(images < SEEDS as usize * 35 / 100, "{images}");
}

#[test]
fn should_give_neurobox_networks_their_own_texts_and_neither_images_nor_mails() {
    let items = WorldItemTable::get();
    let runs = all(true);

    assert!(runs.iter().any(|content| !content.files.is_empty()));

    for content in &runs {
        assert!(content.mails.is_empty());

        for file in &content.files {
            let id = serde_json::from_str::<serde_json::Value>(&file.data.content).unwrap();
            let id = id["chatID"]
                .as_str()
                .or(id["textID"].as_str())
                .unwrap()
                .to_owned();

            assert!(
                items.neurobox_texts.iter().any(|item| item.id == id),
                "{id}"
            );
        }
    }

    // 70% of the computers draw the file branch.
    let with_files = runs
        .iter()
        .filter(|content| !content.files.is_empty())
        .count();

    assert!(with_files > SEEDS as usize * 55 / 100, "{with_files}");
}

#[test]
fn should_only_mail_conversations_of_npc_accounts() {
    let items = WorldItemTable::get();
    let none = RandomContent::draw(&mut Random::new(1), &users(), false, &|_, _| false, &|_| {
        false
    });

    assert!(none.mails.is_empty());

    let mut seen = std::collections::BTreeSet::new();

    for content in all(false) {
        assert!(content.mails.len() <= 2);

        for mail in content.mails {
            let item = items
                .texts
                .iter()
                .find(|item| item.id == mail.conversation)
                .unwrap();

            assert!(item.id.contains("CONV") && !item.only_file);
            assert!(["anna", "ben"].contains(&mail.user.as_str()));

            let (surname, domain) = mail.receive_user.split_once('@').unwrap();

            assert!(!surname.is_empty());
            assert!(
                [".com", ".net", ".org", ".info"]
                    .iter()
                    .any(|suffix| domain.ends_with(suffix)),
                "{domain}"
            );
            assert_eq!(domain, domain.to_lowercase());
            seen.insert(mail.conversation);
        }
    }

    assert!(seen.contains("CONV_CASUAL_M_01"));
    assert!(!seen.contains("CONV_CASUAL_02"));
}
