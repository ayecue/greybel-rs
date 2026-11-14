use mono_random::Random;

use super::context::Context;
use super::functions::{
    PlaceholderList, apply_replacements, conversation_lines, json, json_map, seed_from_ip,
    shuffle_rows, tag_index, text,
};
use crate::model::{FileItem, TagMap};

pub(super) struct Resolver<'a> {
    pub(super) context: &'a Context<'a>,
}

impl Resolver<'_> {
    /// `Router.ResolveMissionRuntimeTags`.
    pub(super) fn resolve_tags(&self, tags: &TagMap) -> TagMap {
        if tags.is_empty() {
            return TagMap::new();
        }

        let mut tags = tags.clone();

        if let Some(remote) = tags.get("$IP_REMOTE").cloned()
            && let Some(network) = tag_index(&remote, "$IP_IDX")
            && let Some(placeholder) = tags.get("$IP_REMLOCAL").cloned()
            && let Some(local) = tag_index(&placeholder, "$IP_LOCAL_IDX")
        {
            let hits = self.remote_local(network, local);

            tags.insert(
                "$IP_REMLOCAL".to_owned(),
                hits.unwrap_or_else(|| "unknown".to_owned()),
            );
        }

        self.resolve_decoys(&mut tags);

        tags
    }

    /// `Router.TryResolveNetworkLocalIp`.
    pub(super) fn remote_local(&self, network: usize, local: usize) -> Option<String> {
        let address = self.context.group.get(network)?;
        let list = if address == self.context.public_ip {
            self.context.local_ips.to_vec()
        } else {
            (self.context.remote_local_ips)(network)
        };

        list.get(local).cloned()
    }

    /// `MissionTextReplacer.ResolveDecoyIpCollisions`.
    pub(super) fn resolve_decoys(&self, tags: &mut TagMap) {
        let keys: Vec<String> = tags
            .keys()
            .filter(|key| {
                key.starts_with("$RANDOM_LOCAL_IP") || key.starts_with("$RANDOM_PUBLIC_IP")
            })
            .cloned()
            .collect();

        if keys.is_empty() {
            return;
        }

        let resolve = |value: &str| {
            if let Some(index) = tag_index(value, "$IP_IDX") {
                self.context.group.get(index).cloned().unwrap_or_default()
            } else if let Some(index) = tag_index(value, "$IP_LOCAL_IDX") {
                self.context
                    .local_ips
                    .get(index)
                    .cloned()
                    .unwrap_or_default()
            } else {
                value.to_owned()
            }
        };
        let mut unavailable = std::collections::HashSet::new();

        for key in ["$IP_LOCAL", "$IP_REMOTE", "$IP_REMLOCAL"] {
            if let Some(value) = tags.get(key) {
                unavailable.insert(resolve(value));
            }
        }

        // The keys are visited in ordinal order (`BTreeMap` order is byte order).
        for key in keys {
            let local = key.starts_with("$RANDOM_LOCAL_IP");
            let mut value = tags[&key].clone();

            if !unavailable.insert(value.clone()) {
                let mut random = Random::new(seed_from_ip(&value));

                loop {
                    value = if local {
                        crate::tags::random_local_ip(&mut random)
                    } else {
                        crate::tags::random_public_ip(&mut random)
                    };

                    if unavailable.insert(value.clone()) {
                        break;
                    }
                }

                tags.insert(key, value);
            }
        }
    }

    /// `FileItem.ResolveText`: applies the tags and the network placeholders.
    pub(super) fn resolve_text(
        &self,
        text: &str,
        file: Option<&FileItem>,
        tags: &TagMap,
    ) -> String {
        let mut text = match file {
            Some(file) => apply_replacements(&shuffle_rows(text, file.row_shuffle_seed), tags),
            None => apply_replacements(text, tags),
        };

        for (index, address) in self.context.group.iter().enumerate() {
            text = text.replace(&format!("$IP_IDX{index}"), address);
        }

        for (index, address) in self.context.local_ips.iter().enumerate() {
            text = text.replace(&format!("$IP_LOCAL_IDX{index}"), address);
        }

        if text.contains("$PASS_IDX") || text.contains("$NPC_IDX") {
            for (index, npc) in self.context.npcs.iter().enumerate() {
                text = text
                    .replace(
                        &format!("$PASS_IDX{index}"),
                        npc.forced_pass.as_deref().unwrap_or(""),
                    )
                    .replace(&format!("$NPC_IDX{index}"), &npc.surname);
            }
        }

        text
    }

    /// `HelperConversation`/`HelperPDF`: the placeholders of `lines` that are IP indexes or NPC indexes with what
    /// they stand for, in the order the game finds them.
    pub(super) fn references(
        &self,
        lines: &[String],
    ) -> (PlaceholderList, PlaceholderList, PlaceholderList) {
        let mut public = Vec::new();
        let mut local = Vec::new();
        let mut npcs = Vec::new();

        for line in lines {
            for (tag, list, hits) in [
                ("$IP_IDX", self.context.group, &mut public),
                ("$IP_LOCAL_IDX", self.context.local_ips, &mut local),
            ] {
                if let Some(at) = line.find(tag)
                    && let Some(key) = line.get(at..at + tag.len() + 1)
                    && let Some(digit) = key.chars().last().and_then(|c| c.to_digit(10))
                    && !hits.iter().any(|(current, _)| current == key)
                    && let Some(address) = list.get(digit as usize)
                {
                    hits.push((key.to_owned(), address.clone()));
                }
            }

            let mut tail = line.as_str();

            for _ in 0..self.context.npcs.len() {
                let Some(at) = tail.find("$NPC_IDX") else {
                    continue;
                };
                let Some(key) = tail.get(at..at + "$NPC_IDX".len() + 1) else {
                    break;
                };

                if let Some(digit) = key.chars().last().and_then(|c| c.to_digit(10))
                    && !npcs.iter().any(|(current, _)| current == key)
                    && let Some(npc) = self.context.npcs.get(digit as usize)
                {
                    npcs.push((key.to_owned(), npc.complete_name()));
                }

                tail = &tail[at + "$NPC_IDX".len() + 1..];
            }
        }

        (public, local, npcs)
    }

    /// `HelperConversation.ToJson` of conversation `id`.
    pub(super) fn conversation_json(
        &self,
        id: &str,
        send: &str,
        receive: &str,
        tags: &TagMap,
    ) -> String {
        let lines = conversation_lines(id);
        let lines: Vec<String> = lines
            .iter()
            .map(|(_, line)| apply_replacements(line, tags))
            .collect();
        let (public, local, npcs) = self.references(&lines);
        let tag_list: Vec<(String, String)> = tags.clone().into_iter().collect();
        // `HelperConversation.ConfigZeroDayIP`.
        let zero_day_ip = if lines.iter().any(|line| line.contains("$IP_ZERODAY")) {
            self.context.zero_day_ip
        } else {
            ""
        };

        format!(
            r#"{{"chatID":{},"users":{{"SEND":{},"RECEIVE":{}}},"idxPublicIPs":{},"idxLocalIPs":{},"idxNpcs":{},"runtimeTags":{},"zeroDayNetworkIP":{}}}"#,
            json(id),
            json(send),
            json(receive),
            json_map(&public),
            json_map(&local),
            json_map(&npcs),
            json_map(&tag_list),
            json(zero_day_ip),
        )
    }

    /// `HelperPDF.ToJson` of text `id`.
    pub(super) fn pdf_json(&self, id: &str, tags: &TagMap) -> String {
        let lines: Vec<String> = text(id)
            .split('\n')
            .map(|line| apply_replacements(line, tags))
            .collect();
        let (public, local, npcs) = self.references(&lines);
        let tag_list: Vec<(String, String)> = tags.clone().into_iter().collect();

        format!(
            r#"{{"textID":{},"imgID":"","idxPublicIPs":{},"idxLocalIPs":{},"idxNpcs":{},"runtimeTags":{}}}"#,
            json(id),
            json_map(&public),
            json_map(&local),
            json_map(&npcs),
            json_map(&tag_list),
        )
    }
}
