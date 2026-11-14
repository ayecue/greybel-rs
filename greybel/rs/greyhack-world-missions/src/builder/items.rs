//! Mission items built from beats: payloads, files, chat logs, and cross information.

use std::collections::HashMap;

use mono_random::Random;

use super::beat::Beat;
use super::build::TagStore;
use super::build_error::BuildError;
use super::runtime_tags::resolve_runtime_tags;
use super::state::State;
use crate::config::{
    ClosureEntry, ClosureType, FileOptionSpec, Modality, NextMode, PayloadType, TemplateOption,
};
use crate::model::{FileItem, ItemKind, MissionItem, TagMap};
use crate::tags::between;

pub(super) fn item_from_payload(
    payload: PayloadType,
    option: &TemplateOption,
    is_evidence: bool,
    random: &mut Random,
    suffix: Option<&str>,
) -> MissionItem {
    match payload {
        PayloadType::ChatLog => {
            let mut item = MissionItem::new(
                &option.text_id,
                ItemKind::ChatLog {
                    force_npc_receive: false,
                    index_npc: -1,
                    random_path: option.chat_log_path_spec.random_path,
                    path: option.chat_log_path_spec.path.clone(),
                },
            );

            item.is_evidence = is_evidence;

            item
        }
        PayloadType::File => file_item(option, is_evidence, random, suffix),
    }
}

/// `ProcMissionBuilder.BuildItemFromOption`.
pub(super) fn build_item_from_option(beat: &Beat<'_>, random: &mut Random) -> Option<MissionItem> {
    let option = beat.option;

    Some(match beat.modality {
        Modality::ChatLog => item_from_payload(PayloadType::ChatLog, option, false, random, None),
        Modality::File => item_from_payload(PayloadType::File, option, false, random, None),
        Modality::Email => MissionItem::new(
            &option.text_id,
            ItemKind::Mails {
                force_npc_receive: false,
                index_npc: -1,
                use_network_npc_sender: true,
                title_id: option.email_title_text_id.clone(),
            },
        ),
        Modality::CrossInfo => {
            let spec = option.cross_info_spec.as_ref()?;
            let suffix = (spec.current_type == PayloadType::File).then_some("xcur");

            item_from_payload(spec.current_type, option, false, random, suffix)
        }
    })
}

/// `ProcMissionBuilder.BuildSupportChatLogItem`.
pub(super) fn support_chat_log_item(beat: &Beat<'_>, tags: &TagMap) -> Option<MissionItem> {
    let option = beat.option;

    option.generate_support_chat_log.then(|| {
        let mut item = MissionItem::new(
            &option.support_chat_log_text_id,
            ItemKind::ChatLog {
                force_npc_receive: false,
                index_npc: -1,
                random_path: option.support_chat_log_path_spec.random_path,
                path: option.support_chat_log_path_spec.path.clone(),
            },
        );

        item.runtime_tags.clone_from(tags);

        item
    })
}

/// `ProcMissionBuilder.BuildClosureItem`.
pub(super) fn closure_item(entry: &ClosureEntry, tags: TagMap, random: &mut Random) -> MissionItem {
    let payload = match entry.kind {
        ClosureType::File => PayloadType::File,
        ClosureType::ChatLog => PayloadType::ChatLog,
    };
    let mut item = item_from_payload(payload, &entry.option, true, random, None);

    item.runtime_tags = tags;

    item
}

/// `ProcMissionBuilder.BuildFileItem`.
pub(super) fn file_item(
    option: &TemplateOption,
    is_evidence: bool,
    random: &mut Random,
    suffix: Option<&str>,
) -> MissionItem {
    let spec = &option.file_spec;
    let automatic = auto_file_name(spec, random, suffix);
    let label = spec.file_name.as_str();
    let use_automatic = spec.random_file_name || label.trim().is_empty();
    let row_shuffle_seed = random.next_int();
    let mut item = MissionItem::new(
        &option.text_id,
        ItemKind::TextFile(Box::new(FileItem {
            row_shuffle_seed,
            file_name: if use_automatic {
                automatic
            } else {
                label.trim().to_owned()
            },
            random_path: spec.random_path,
            is_pdf: spec.is_pdf,
            is_binary: spec.is_binary,
            is_script: spec.is_script,
            allow_import: spec.allow_import,
            companion_files: spec.companion_files.clone(),
            is_image: spec.is_image,
            path: spec.path.clone(),
            symlink_target: spec.symlink_target.clone(),
            owner: spec.owner.clone(),
            permissions: spec.permissions.clone(),
            directory_permissions: spec.directory_permissions.clone(),
            encrypt_with_owner_password: spec.encrypt_with_owner_password,
            encryption_password: spec.encryption_password.clone(),
            is_web: false,
            raw_content: if spec.is_binary {
                spec.raw_content.clone()
            } else {
                String::new()
            },
        })),
    );

    item.is_evidence = is_evidence;

    item
}

/// `ProcMissionBuilder.BuildAutoFileName`.
pub(super) fn auto_file_name(
    spec: &FileOptionSpec,
    random: &mut Random,
    suffix: Option<&str>,
) -> String {
    let mut name = format!("file{}", between(random, 0, 1000));

    if let Some(suffix) = suffix.filter(|suffix| !suffix.is_empty()) {
        name = format!("{name}_{}", suffix.trim());
    }

    if spec.is_script {
        return name;
    }

    let extension = if spec.is_binary {
        "bin"
    } else if spec.is_pdf {
        "pdf"
    } else if spec.is_image {
        "jpg"
    } else {
        "txt"
    };

    format!("{name}.{extension}")
}

/// `ProcMissionBuilder.CollectCrossInfoInjectedItems`: what a cross-information beat adds to the beat after it.
pub(super) fn collect_cross_info_items(
    state: &mut State<'_>,
    beats: &[Beat<'_>],
    index: usize,
    closure: &Beat<'_>,
    common: &mut [Option<TagStore>],
    extras: &mut HashMap<i32, Vec<MissionItem>>,
) -> Result<(), BuildError> {
    let beat = &beats[index];

    if beat.modality != Modality::CrossInfo {
        return Ok(());
    }

    let spec = beat
        .option
        .cross_info_spec
        .as_ref()
        .ok_or(BuildError::MissingTemplate)?;
    let has_next = index + 1 < beats.len();

    if spec.next_mode == NextMode::ReplaceNextBeat && has_next {
        common[index + 1] = common[index].clone();

        return Ok(());
    }

    let target = if has_next { &beats[index + 1] } else { closure };
    let after = (target.index != closure.index).then(|| beats.get(index + 2).unwrap_or(closure));
    let tags = resolve_runtime_tags(state, target, after, &spec.next_option, &mut common[index])?;
    let suffix = (spec.next_mode == NextMode::CoexistWithNextBeat
        && spec.next_type == PayloadType::File)
        .then_some("xnext");
    let mut item = item_from_payload(
        spec.next_type,
        &spec.next_option,
        false,
        state.random,
        suffix,
    );

    item.runtime_tags = tags;
    extras.entry(target.index).or_default().push(item);

    Ok(())
}
