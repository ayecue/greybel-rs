use greyhack_behaviour::{FileKind, Gender, Word};
use greyhack_world_assets::{WorldItem, WorldItemTable};
use mono_random::Random;

use super::{ContentFile, ConversationMail};
use crate::generation::network::web_domain;

/// What `Router.GenerateRandomContent` adds to a computer.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RandomContent {
    /// Files and images in the order they are added; later ones replace earlier ones of the same
    /// name.
    pub files: Vec<ContentFile>,
    pub mails: Vec<ConversationMail>,
}

impl RandomContent {
    /// Draws the random content of a computer (`Router.GenerateRandomContent`) with `random`, which
    /// the game seeds with the network seed plus the seed of the computer's LAN address. `users` are
    /// the computer's accounts, root first. `exists` tells whether a user's home already has an
    /// entry of a name, `has_persona` whether an account belongs to an NPC. Neurobox networks use
    /// other texts and have no images or mails.
    ///
    /// The files do not depend on `exists` or `has_persona` in the random numbers they draw, only
    /// the mails do (through `has_persona`).
    pub(crate) fn draw(
        random: &mut Random,
        users: &[String],
        neurobox: bool,
        exists: &dyn Fn(&str, &str) -> bool,
        has_persona: &dyn Fn(&str) -> bool,
    ) -> Self {
        let items = WorldItemTable::get();
        let mut content = Self::default();
        let mut images = 2;
        let mails = if neurobox { 7 } else { 5 };
        // `Computer.GetRandomUser` skips root unless it is the only account.
        let user = if users.len() == 1 {
            &users[0]
        } else {
            &users[1 + random.next_below(users.len() - 1)]
        };

        if random.next_below(10) < mails {
            images += 2;

            for _ in 0..1 + random.next_below(2) {
                if let Some(file) = random_file(random, user, neurobox, items)
                    && !exists(user, &file.name)
                    && !content
                        .files
                        .iter()
                        .any(|entry| entry.user == file.user && entry.name == file.name)
                {
                    content.files.push(file);
                }
            }
        }

        if neurobox {
            return content;
        }

        if random.next_below(10) < images {
            for _ in 0..1 + random.next_below(2) {
                let image = random_image(random, user, items);

                content
                    .files
                    .retain(|entry| entry.user != image.user || entry.name != image.name);
                content.files.push(image);
            }
        }

        if random.next_below(10) < mails {
            for _ in 0..1 + random.next_below(2) {
                if let Some(mail) = random_mail(random, user, items, has_persona) {
                    content.mails.push(mail);
                }
            }
        }

        content
    }
}

/// `ItemsWorldGen.GetRandomImage`.
fn random_image(random: &mut Random, user: &str, items: &WorldItemTable) -> ContentFile {
    let item = &items.images[random.next_below(items.images.len())];
    let name = if item.name.is_empty() {
        format!("Img_{}.jpg", random.next_below(5000))
    } else {
        item.name.clone()
    };

    ContentFile::of_kind(user, name, FileKind::Image, String::new())
}

/// `ItemsWorldGen.GetRandomFile`. Mail-only conversations give no file.
fn random_file(
    random: &mut Random,
    user: &str,
    neurobox: bool,
    items: &WorldItemTable,
) -> Option<ContentFile> {
    let list = if neurobox {
        &items.neurobox_texts
    } else {
        &items.texts
    };
    let item: &WorldItem = &list[random.next_below(list.len())];

    if item.id.contains("CASUAL_M") {
        return None;
    }

    let (name, kind, content) = if item.id.contains("CONV") {
        let gender = if random.next_double() > 0.5 {
            Gender::Male
        } else {
            Gender::Female
        };
        let receive = gender.first_name_with_index(|length| random.next_below(length));
        let name = format!("chat{}.log", random.next_below(1000));

        (
            name,
            FileKind::ChatLog,
            format!(
                r#"{{"chatID":{},"users":{{"SEND":{},"RECEIVE":{}}},"idxPublicIPs":{{}},"idxLocalIPs":{{}},"idxNpcs":{{}},"runtimeTags":{{}},"zeroDayNetworkIP":""}}"#,
                json(&item.id),
                json(user),
                json(&receive)
            ),
        )
    } else if item.id.contains("FILE") {
        let content = format!(
            r#"{{"textID":{},"imgID":"","idxPublicIPs":{{}},"idxLocalIPs":{{}},"idxNpcs":{{}},"runtimeTags":{{}}}}"#,
            json(&item.id)
        );
        let name = if item.name.is_empty() {
            format!("file_{}.pdf", random.next_below(5000))
        } else {
            item.name.clone()
        };

        (name, FileKind::Pdf, content)
    } else {
        return None;
    };

    Some(ContentFile::of_kind(user, name, kind, content))
}

/// `ItemsWorldGen.GenRandomMail`.
fn random_mail(
    random: &mut Random,
    user: &str,
    items: &WorldItemTable,
    has_persona: &dyn Fn(&str) -> bool,
) -> Option<ConversationMail> {
    let item = &items.texts[random.next_below(items.texts.len())];

    if !item.id.contains("CONV") || item.only_file || !has_persona(user) {
        return None;
    }

    let surname = Word::Surname.generate_with_index(|length| random.next_below(length));
    let company = Word::Corporation.generate_with_index(|length| random.next_below(length));
    let domain = web_domain(&company, random);

    Some(ConversationMail {
        user: user.to_owned(),
        conversation: item.id.clone(),
        receive_user: format!("{surname}@{domain}"),
    })
}

fn json(text: &str) -> String {
    serde_json::to_string(text).expect("a string is JSON")
}
