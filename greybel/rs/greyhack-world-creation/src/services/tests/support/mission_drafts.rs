use greyhack_behaviour::ROOT_USER;
use greyhack_world_missions::{Assignment, PreGenNpc};

use crate::{ComputerDraft, User};

pub(super) fn draft(users: &[&str]) -> ComputerDraft {
    let mut draft = ComputerDraft::new("pc", "192.168.0.2");

    for name in std::iter::once(ROOT_USER).chain(users.iter().copied()) {
        draft.users.push(User::new(name, "pass1"));
    }

    draft
}

pub(super) fn assignment(replacement: Option<f64>) -> Assignment {
    Assignment {
        local_ip: "192.168.0.2".to_owned(),
        computer: 0,
        replacement,
        npc_in_lan: true,
    }
}

pub(super) fn npc(surname: &str) -> PreGenNpc {
    PreGenNpc {
        surname: surname.to_owned(),
        ..PreGenNpc::default()
    }
}
