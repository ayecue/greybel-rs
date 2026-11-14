use greyhack_behaviour::{
    FileData, FileKind, Gender, LogAction, LogEntry, ROOT_USER, Word, format_date, parse_date,
    web_domain,
};
use greyhack_world_assets::{GameTextTable, content_file};
use mono_random::Random;

use super::computer::ComputerContent;
use super::context::Context;
use super::mission_file::MissionFile;
use super::mission_mail::MissionMail;
use super::resolver::Resolver;
use crate::config::{Archetype, CompanionFileSpec};
use crate::model::{ComputerMission, FileItem, ItemKind, MissionItem, TagMap};
use crate::tags::{between, next_word};

pub(super) fn text(id: &str) -> String {
    GameTextTable::english()
        .get(id)
        .map_or_else(|| id.to_owned(), str::to_owned)
}

pub(super) fn json(value: &str) -> String {
    serde_json::to_string(value).expect("a string is JSON")
}

pub(super) fn json_map(entries: &[(String, String)]) -> String {
    let body: Vec<String> = entries
        .iter()
        .map(|(key, value)| format!("{}:{}", json(key), json(value)))
        .collect();

    format!("{{{}}}", body.join(","))
}

/// `Router.TryParseTagIndex`.
pub(super) fn tag_index(placeholder: &str, prefix: &str) -> Option<usize> {
    placeholder.strip_prefix(prefix)?.parse().ok()
}

/// `IP.GetSeedFromIP`.
pub fn seed_from_ip(address: &str) -> i32 {
    address
        .parse::<std::net::Ipv4Addr>()
        .map_or(0, |address| i32::from_le_bytes(address.octets()))
}

/// `MissionTextReplacer.ApplyKeyReplacements`: replaces the longest keys first.
pub(super) fn apply_replacements(text: &str, tags: &TagMap) -> String {
    let mut keys: Vec<&String> = tags.keys().collect();

    keys.sort_by_key(|key| std::cmp::Reverse(key.len()));

    keys.into_iter().fold(text.to_owned(), |text, key| {
        text.replace(key.as_str(), &tags[key])
    })
}

/// `FileItem.ShuffleRows`.
pub(super) fn shuffle_rows(text: &str, seed: i32) -> String {
    const OPEN: &str = "$SHUFFLE_ROWS\n";
    const CLOSE: &str = "\n$END_SHUFFLE_ROWS";

    if !text.contains("$SHUFFLE_ROWS") {
        return text.to_owned();
    }

    let mut random = Random::new(seed);
    let mut result = String::new();
    let mut tail = text;

    while let Some(start) = tail.find(OPEN) {
        let Some(end) = tail[start + OPEN.len()..].find(CLOSE) else {
            break;
        };
        let body_start = start + OPEN.len();
        let body = &tail[body_start..body_start + end];
        let mut rows: Vec<&str> = body.split('\n').collect();

        for at in (1..rows.len()).rev() {
            let other = random.next_below(at + 1);

            rows.swap(at, other);
        }

        result.push_str(&tail[..start]);
        result.push_str(&rows.join("\n"));
        tail = &tail[body_start + end + CLOSE.len()..];
    }

    result.push_str(tail);

    result
}

/// Placeholders with what they stand for.
pub(super) type PlaceholderList = Vec<(String, String)>;

/// The lines of a conversation text: speaker (0 sends, 1 receives) and text.
pub(super) fn conversation_lines(id: &str) -> Vec<(u8, String)> {
    #[derive(serde::Deserialize)]
    struct Line {
        #[serde(rename = "idNpc")]
        speaker: u8,
        text: String,
    }

    serde_json::from_str::<Vec<Line>>(&text(id)).map_or_else(
        |_| Vec::new(),
        |lines| {
            lines
                .into_iter()
                .map(|line| (line.speaker, line.text))
                .collect()
        },
    )
}

/// `Router.BuildMissionMailAddress`.
pub(super) fn mission_mail_address(forced_surname: &str, random: &mut Random) -> String {
    let surname = if forced_surname.is_empty() {
        next_word(Word::Surname, random)
    } else {
        forced_surname.to_owned()
    };
    let company = next_word(Word::Corporation, random);

    format!("{surname}@{}", web_domain(&company, random))
}

pub(super) fn date_text(seconds: u64) -> String {
    format_date(seconds)
}

/// What `PostGenMission` gives `computer` of a mission network, or `None` when the game does not (a computer without
/// accounts). `proc_archetype` objectives are prepared first.
pub fn computer_content(
    context: &Context<'_>,
    computer: &ComputerMission,
) -> Option<ComputerContent> {
    if context.users.is_empty() {
        return None;
    }

    let resolver = Resolver { context };
    let mut random = Random::new(context.seed);
    let owner = mission_owner(context, computer, &mut random);
    let owner_password = context
        .users
        .iter()
        .find(|(name, _)| *name == owner)
        .map_or_else(String::new, |(_, password)| password.clone());
    let home = if owner == ROOT_USER {
        "/root".to_owned()
    } else {
        format!("/home/{owner}")
    };
    let mut content = ComputerContent {
        owner: owner.clone(),
        ..ComputerContent::default()
    };
    let objective = computer
        .procedural_archetype
        .map(|archetype| prepare_objective(context, archetype, &owner, &home, &mut content));

    for item in &computer.items {
        let mut tags = resolver.resolve_tags(&item.runtime_tags);

        if item.is_evidence
            && let Some(objective) = &objective
        {
            tags.extend(objective.clone());
        }

        match &item.kind {
            ItemKind::ChatLog {
                force_npc_receive,
                index_npc,
                random_path,
                path,
            } => {
                let gender = if random.next_double() > 0.5 {
                    Gender::Male
                } else {
                    Gender::Female
                };
                let receive = if *force_npc_receive {
                    context
                        .npcs
                        .get(usize::try_from(*index_npc).unwrap_or(usize::MAX))
                        .map(|npc| npc.name.clone())
                        .unwrap_or_default()
                } else {
                    gender.first_name_with_index(|length| random.next_below(length))
                };
                let body = resolver.conversation_json(&item.id, &owner, &receive, &tags);
                let name = format!("chat{}.log", between(&mut random, 0, 1000));
                let mut data = FileData::text(&body);

                data.kind = FileKind::ChatLog;
                data.default_content = true;

                let folder = if *random_path {
                    format!("{home}{path}").replace("//", "/")
                } else {
                    path.clone()
                };

                content.files.push(MissionFile {
                    folder,
                    name,
                    data,
                    owner: owner.clone(),
                    permissions: None,
                    symlink: String::new(),
                    timestamp: None,
                    folder_permissions: None,
                    create_folder: false,
                });
            }
            ItemKind::TextFile(file) => {
                text_file(
                    &resolver,
                    &mut content,
                    item,
                    file,
                    &tags,
                    &owner,
                    &owner_password,
                    &home,
                );
            }
            ItemKind::SystemLog {
                action,
                port,
                to_computer_idx,
                is_receiving_conn,
            } => {
                let mut ip = context.public_ip.to_owned();
                let mut other = usize::try_from(*to_computer_idx)
                    .ok()
                    .and_then(|index| context.group.get(index))
                    .cloned()
                    .unwrap_or_default();

                if *is_receiving_conn {
                    std::mem::swap(&mut ip, &mut other);
                }

                content.log.push(LogEntry {
                    action: LogAction::from_code(i64::from(*action)).unwrap_or(LogAction::Open),
                    ip,
                    fecha: date_text(context.now),
                    puerto: i64::from(*port),
                    file: "system.log".to_owned(),
                    bounce_ip: other,
                    player_net_id: String::new(),
                    tutorial: false,
                    token_trace: String::new(),
                });
            }
            ItemKind::Mails {
                force_npc_receive,
                index_npc,
                use_network_npc_sender,
                title_id,
            } => {
                let outsider = if *use_network_npc_sender {
                    mission_mail_address("", &mut random)
                } else {
                    let preset = if *force_npc_receive {
                        context
                            .npcs
                            .get(usize::try_from(*index_npc).unwrap_or(usize::MAX))
                            .map(|npc| npc.surname.clone())
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };

                    mission_mail_address(&preset, &mut random)
                };
                let lines = conversation_lines(&item.id)
                    .into_iter()
                    .map(|(speaker, line)| {
                        let line = apply_replacements(&line, &tags);

                        (speaker, mail_text(&resolver, &line, &tags))
                    })
                    .collect();

                content.mails.push(MissionMail {
                    title: if title_id.is_empty() {
                        "RE: stuff".to_owned()
                    } else {
                        text(title_id)
                    },
                    outsider,
                    outsider_is_sender: *use_network_npc_sender,
                    lines,
                });
            }
            ItemKind::Service { .. } | ItemKind::Firewall(_) => {}
        }
    }

    Some(content)
}

/// `HelperConversation.ResolveText` after the tags: placeholders of the network.
pub(super) fn mail_text(resolver: &Resolver<'_>, line: &str, tags: &TagMap) -> String {
    let (public, local, npcs) = resolver.references(&[apply_replacements(line, tags)]);
    let mut text = apply_replacements(line, tags);

    for (key, value) in public.into_iter().chain(local).chain(npcs) {
        text = text.replace(&key, &value);
    }

    text.replace("$IP_ZERODAY", resolver.context.zero_day_ip)
}

/// `Router.GetMissionOwnerUser`: the account the files of the computer belong to.
pub(super) fn mission_owner(
    context: &Context<'_>,
    computer: &ComputerMission,
    random: &mut Random,
) -> String {
    let random_user = if context.users.len() == 1 {
        context.users[0].0.clone()
    } else {
        context.users[1 + random.next_below(context.users.len() - 1)]
            .0
            .clone()
    };

    usize::try_from(computer.clone_npc_pc_idx)
        .ok()
        .and_then(|index| context.npcs.get(index))
        .filter(|npc| !npc.surname.is_empty())
        .and_then(|npc| {
            context
                .users
                .iter()
                .find(|(name, _)| *name == npc.surname)
                .map(|(name, _)| name.clone())
        })
        .unwrap_or(random_user)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn text_file(
    resolver: &Resolver<'_>,
    content: &mut ComputerContent,
    item: &MissionItem,
    file: &FileItem,
    tags: &TagMap,
    owner: &str,
    owner_password: &str,
    home: &str,
) {
    if let Some(password) = tags.get("$HISTORICAL_PASSWORD") {
        content.passwords.push(password.clone());
    }

    let apply = |text: &str| apply_replacements(&shuffle_rows(text, file.row_shuffle_seed), tags);
    let name = apply(&file.file_name);
    let body = if file.is_pdf {
        resolver.pdf_json(&item.id, tags)
    } else if file.is_web {
        apply(content_file(&item.id))
    } else if file.is_binary {
        apply(&file.raw_content)
    } else {
        resolver.resolve_text(&text(&item.id), Some(file), tags)
    };
    let mut data = if file.is_pdf {
        FileData {
            kind: FileKind::Pdf,
            ..FileData::binary(FileData::binary_size(&name, false))
        }
    } else if file.is_script {
        FileData::script(&body)
    } else if file.is_image {
        FileData {
            kind: FileKind::Image,
            ..FileData::binary(FileData::binary_size(&name, false))
        }
    } else if file.is_binary {
        FileData::binary(FileData::binary_size(&name, false))
    } else {
        FileData::default()
    };

    if !file.is_script {
        data.content.clone_from(&body);

        if !data.is_binary() {
            data.size = body.encode_utf16().count() as u64;
        }
    }

    data.default_content = true;
    data.allow_import = file.allow_import;

    if file.encrypt_with_owner_password {
        data.encrypt_password = owner_password.to_owned();
    } else if !file.encryption_password.is_empty() {
        data.encrypt_password = apply(&file.encryption_password);
    }

    let timestamp = tags
        .get("$FILE_TIMELINE_TIMESTAMP")
        .map(|value| parse_date(&value.replacen(' ', " - ", 1)));
    let folder = if file.random_path {
        format!("{home}{}", file.path).replace("//", "/")
    } else {
        file.path.clone()
    };
    let symlink = if file.symlink_target.is_empty() {
        String::new()
    } else {
        resolver.resolve_text(&file.symlink_target, Some(file), tags)
    };
    let file_owner = if file.owner.is_empty() {
        owner.to_owned()
    } else {
        file.owner.clone()
    };

    content.files.push(MissionFile {
        folder: folder.clone(),
        name,
        data,
        owner: file_owner,
        permissions: (!file.permissions.is_empty()).then(|| file.permissions.clone()),
        symlink,
        timestamp,
        folder_permissions: (!file.directory_permissions.is_empty())
            .then(|| file.directory_permissions.clone()),
        create_folder: !file.directory_permissions.is_empty(),
    });

    for companion in &file.companion_files {
        content.files.push(companion_file(
            resolver, file, companion, &folder, owner, tags,
        ));
    }
}

pub(super) fn companion_file(
    resolver: &Resolver<'_>,
    file: &FileItem,
    companion: &CompanionFileSpec,
    folder: &str,
    owner: &str,
    tags: &TagMap,
) -> MissionFile {
    let body = if companion.text_id.is_empty() {
        companion.content.clone()
    } else {
        resolver.resolve_text(&text(&companion.text_id), Some(file), tags)
    };
    let mut data = FileData::text(&body);

    data.default_content = true;

    if !companion.encryption_password.is_empty() {
        data.encrypt_password = apply_replacements(
            &shuffle_rows(&companion.encryption_password, file.row_shuffle_seed),
            tags,
        );
    }

    MissionFile {
        folder: folder.to_owned(),
        name: companion.file_name.clone(),
        data,
        owner: owner.to_owned(),
        permissions: (!companion.permissions.is_empty()).then(|| companion.permissions.clone()),
        symlink: String::new(),
        timestamp: None,
        folder_permissions: None,
        create_folder: false,
    }
}

/// `Router.PrepareProceduralObjective`: the files that make the goal of the mission checkable, and the tags it
/// gives the evidence.
pub(super) fn prepare_objective(
    context: &Context<'_>,
    archetype: Archetype,
    owner: &str,
    home: &str,
    content: &mut ComputerContent,
) -> TagMap {
    let mut random = Random::new(context.objective_seed);
    let mut tags = TagMap::new();
    let mut target_user = owner.to_owned();
    let add = |content: &mut ComputerContent,
               path: &str,
               body: &str,
               file_owner: &str,
               key: Option<&str>| {
        let (folder, name) = path.rsplit_once('/').unwrap_or(("", path));
        let mut data = FileData::text(body);

        data.default_content = true;

        if let Some(key) = key {
            data.encrypt_password = key.to_owned();
        }

        content.files.push(MissionFile {
            folder: if folder.is_empty() {
                "/".to_owned()
            } else {
                folder.to_owned()
            },
            name: name.to_owned(),
            data,
            owner: file_owner.to_owned(),
            permissions: Some("-rw-------".to_owned()),
            symlink: String::new(),
            timestamp: None,
            folder_permissions: None,
            create_folder: true,
        });
    };

    match archetype {
        Archetype::Fraud => target_user = ROOT_USER.to_owned(),
        Archetype::CorporateEspionage => add(
            content,
            &format!("{home}/tender_dossier.txt"),
            &text("PROCM_OBJECTIVE_DOSSIER"),
            owner,
            None,
        ),
        Archetype::Ransomware => {
            let key = format!("restore{}", between(&mut random, 10_000_000, 99_999_999));

            add(
                content,
                &format!("{home}/client_backup.enc"),
                &text("PROCM_OBJECTIVE_BACKUP"),
                owner,
                Some(&key),
            );
            add(content, "/root/recovery_key.txt", &key, ROOT_USER, None);
        }
        Archetype::Extortion => add(
            content,
            &format!("{home}/publication_package.txt"),
            &text("PROCM_OBJECTIVE_PUBLICATION"),
            owner,
            None,
        ),
        Archetype::EvidenceFabrication => {
            let original = format!("{:08X}", random.next_int());
            let mut fake;

            loop {
                fake = format!("{:08X}", random.next_int());

                if fake != original {
                    break;
                }
            }

            tags.insert("$ORIGINAL_SESSION".to_owned(), original);
            tags.insert("$FABRICATED_SESSION".to_owned(), fake.clone());
            add(
                content,
                &format!("{home}/incident_report.txt"),
                &text("PROCM_OBJECTIVE_INCIDENT").replace("$FABRICATED_SESSION", &fake),
                owner,
                None,
            );
        }
        Archetype::Sabotage | Archetype::IdentityTheft | Archetype::AccessSales => {}
    }

    tags.insert("$TARGET_USER".to_owned(), target_user);

    tags
}

/// The bounce entries of the main router of a network (`NetworkLan.ConfigureRouterLogs`): one for each system log
/// item that bounces, from the computer's local address to the network it bounces to.
pub fn router_log(
    group: &[String],
    mission_computers: &[(String, &ComputerMission)],
    now: u64,
) -> Vec<LogEntry> {
    let mut entries = Vec::new();

    for (local_ip, computer) in mission_computers {
        for item in &computer.items {
            if let ItemKind::SystemLog {
                action,
                port,
                to_computer_idx,
                ..
            } = &item.kind
                && *action == LogAction::Bounce as u8
            {
                entries.push(LogEntry {
                    action: LogAction::Bounce,
                    ip: local_ip.clone(),
                    fecha: date_text(now),
                    puerto: i64::from(*port),
                    file: "system.log".to_owned(),
                    bounce_ip: usize::try_from(*to_computer_idx)
                        .ok()
                        .and_then(|index| group.get(index))
                        .cloned()
                        .unwrap_or_default(),
                    player_net_id: String::new(),
                    tutorial: false,
                    token_trace: String::new(),
                });
            }
        }
    }

    entries
}
