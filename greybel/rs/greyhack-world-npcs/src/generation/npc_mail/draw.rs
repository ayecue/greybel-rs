//! Draws the mails of an NPC account (`Persona.GenerateMailsContents`).

use greyhack_behaviour::SECONDS_PER_HOUR;
use mono_random::Random;

use super::{Administrator, Colleague, Delivery, Owner};
use crate::{CompanyAlign, JobRole, Word, WorkPerformance};

pub(super) const BREAK_SUBJECTS: [&str; 3] = [
    "coffee during the break?",
    "I have a break later",
    "I will be in the break room",
];
pub(super) const STARTER_MISSION_SUBJECT: &str = "something suspicious";
pub(super) const SPAM_SUBJECTS: [&str; 3] = [
    "Don't miss this opportunity!",
    "a discount for you!",
    "You can't miss these offers!",
];

/// Draws the mails of `owner` among `network` (`NetworkLan.GetNpcInfos` in order) and the other NPCs
/// they involve, in the order the game puts them into the mailboxes. `text` returns a game text.
///
/// `mission_contact` is the address the starter mission mail names in a network with a mission
/// (`NetworkLan.missionCreated`); without one that mail takes no random numbers.
///
/// The game stops with an exception where it indexes an empty list (a network without another NPC,
/// a mail of a network without administrator). Those steps deliver nothing here.
pub(crate) fn contents(
    random: &mut Random,
    owner: &Owner,
    network: &[Colleague],
    administrator: Option<&Administrator>,
    mission_contact: Option<&str>,
    text: &dyn Fn(&str) -> String,
) -> Vec<Delivery> {
    let mut deliveries = Vec::new();
    let mut others: Vec<&Colleague> = network.iter().collect();

    work_performance(
        random,
        owner,
        network,
        &mut others,
        administrator,
        text,
        &mut deliveries,
    );
    if let Some(contact) = mission_contact {
        starter_mission(random, owner, &others, contact, text, &mut deliveries);
    }

    schedule(random, owner, &others, text, &mut deliveries);
    spam(random, owner, text, &mut deliveries);

    if owner.company_align != CompanyAlign::None {
        zero_day(random, owner, text, &mut deliveries);
    }

    deliveries
}

/// `Persona.GenerateMailWorkPerformance`. Removes the owner from `others`
/// (`npcInfos.Remove`) when a mail is drawn.
fn work_performance(
    random: &mut Random,
    owner: &Owner,
    network: &[Colleague],
    others: &mut Vec<&Colleague>,
    administrator: Option<&Administrator>,
    text: &dyn Fn(&str) -> String,
    deliveries: &mut Vec<Delivery>,
) {
    let mut bad = Vec::new();
    let mut good = Vec::new();

    for colleague in network {
        if colleague.job_role == JobRole::Admin {
            continue;
        }

        if colleague.work_performance > WorkPerformance::Good {
            good.push(colleague.first_name.clone());
        } else if colleague.work_performance == WorkPerformance::VeryPoor {
            bad.push(colleague.first_name.clone());
        }
    }

    let sending = random.next_below(2) == 1;
    let is_good = random.next_below(2) == 1;
    let names = if is_good { &mut good } else { &mut bad };

    if names.is_empty() {
        return;
    }

    others.retain(|colleague| colleague.surname != owner.colleague.surname);
    remove_first(names, &owner.colleague.surname);

    let Some(other) = pick(random, others) else {
        return;
    };

    remove_first(names, &other.surname);

    let key = if is_good { "GOOD" } else { "BAD" };
    let number = 1 + random.next_below(14);
    let message = text(&format!("WORKMAIL_{key}_{number}"));

    if names.is_empty() {
        return;
    }

    let about = names[random.next_below(names.len())].clone();
    let message = message.replace("$NPC", &about);
    let subject = format!("about {about}");

    deliveries.push(Delivery {
        to: owner.colleague.address.clone(),
        other: other.address.clone(),
        subject: subject.clone(),
        text: message.clone(),
        sending,
    });
    deliveries.push(Delivery {
        to: other.address.clone(),
        other: owner.colleague.address.clone(),
        subject,
        text: message,
        sending: !sending,
    });

    if owner.colleague.job_role != JobRole::Admin
        && let Some(administrator) = administrator
    {
        admin_performance(
            random,
            owner,
            &other.address,
            administrator,
            text,
            deliveries,
        );
    }
}

/// `Persona.GenerateMailAdminPerformance` for the mailbox `other`.
fn admin_performance(
    random: &mut Random,
    owner: &Owner,
    other: &str,
    administrator: &Administrator,
    text: &dyn Fn(&str) -> String,
    deliveries: &mut Vec<Delivery>,
) {
    if random.next_below(3) == 0 {
        return;
    }

    let stats = &administrator.stats;
    let (key, variants) = if stats.is_super_admin_by_stats() {
        ("SUPER", 4)
    } else if stats.work_performance > WorkPerformance::Good {
        ("GOOD", 7)
    } else if stats.work_performance == WorkPerformance::VeryPoor {
        ("BAD", 7)
    } else {
        return;
    };
    let sending = random.next_below(2) == 1;
    let number = 1 + random.next_below(variants);
    let message =
        text(&format!("WORKMAIL_ADMIN_{key}_{number}")).replace("$NPC", &administrator.surname);
    let subject = "about the admin";

    deliveries.push(Delivery {
        to: owner.colleague.address.clone(),
        other: other.to_owned(),
        subject: subject.to_owned(),
        text: message.clone(),
        sending,
    });
    deliveries.push(Delivery {
        to: other.to_owned(),
        other: owner.colleague.address.clone(),
        subject: subject.to_owned(),
        text: message,
        sending: !sending,
    });
}

/// `Persona.GenerateMailSchedule`.
/// `Persona.GenerateMailStarterMission`: in four of five mission networks, a colleague and the owner write to each
/// other about something suspicious at `contact`.
fn starter_mission(
    random: &mut Random,
    owner: &Owner,
    others: &[&Colleague],
    contact: &str,
    text: &dyn Fn(&str) -> String,
    deliveries: &mut Vec<Delivery>,
) {
    if random.next_below(5) == 0 {
        return;
    }

    let Some(other) = pick(random, others) else {
        return;
    };
    let sending = random.next_below(2) == 1;
    let number = 1 + random.next_below(7);
    let message = text(&format!("WORKMAIL_MISSIONS_{number}")).replace("$LOCAL_IP", contact);

    deliveries.push(Delivery {
        to: owner.colleague.address.clone(),
        other: other.address.clone(),
        subject: STARTER_MISSION_SUBJECT.to_owned(),
        text: message.clone(),
        sending,
    });
    deliveries.push(Delivery {
        to: other.address.clone(),
        other: owner.colleague.address.clone(),
        subject: STARTER_MISSION_SUBJECT.to_owned(),
        text: message,
        sending: !sending,
    });
}

fn schedule(
    random: &mut Random,
    owner: &Owner,
    others: &[&Colleague],
    text: &dyn Fn(&str) -> String,
    deliveries: &mut Vec<Delivery>,
) {
    if random.next_below(5) < 3 {
        return;
    }

    let Some(other) = pick(random, others) else {
        return;
    };
    let subject = BREAK_SUBJECTS[random.next_below(BREAK_SUBJECTS.len())];
    let number = 1 + random.next_below(5);
    let message = text(&format!("WORKMAIL_BREAK_{number}")).replace(
        "$TIME",
        &format!(
            "{:02}:{:02}",
            owner.break_start / SECONDS_PER_HOUR % 24,
            owner.break_start / 60 % 60
        ),
    );

    deliveries.push(Delivery {
        to: owner.colleague.address.clone(),
        other: other.address.clone(),
        subject: subject.to_owned(),
        text: message.clone(),
        sending: true,
    });
    deliveries.push(Delivery {
        to: other.address.clone(),
        other: owner.colleague.address.clone(),
        subject: subject.to_owned(),
        text: message,
        sending: false,
    });
}

/// `Persona.GenerateMailSpam`.
fn spam(
    random: &mut Random,
    owner: &Owner,
    text: &dyn Fn(&str) -> String,
    deliveries: &mut Vec<Delivery>,
) {
    for _ in 0..random.next_below(4) {
        let subject = SPAM_SUBJECTS[random.next_below(SPAM_SUBJECTS.len())];
        let number = 1 + random.next_below(5);
        let message = text(&format!("WORKMAIL_SPAM_{number}"));
        let company = Word::Corporation.generate_with_index(|length| random.next_below(length));

        deliveries.push(Delivery {
            to: owner.colleague.address.clone(),
            other: format!("no-reply@{company}.com"),
            subject: subject.to_owned(),
            text: message,
            sending: false,
        });
    }
}

/// `Persona.GenerateMailZeroDay`.
fn zero_day(
    random: &mut Random,
    owner: &Owner,
    text: &dyn Fn(&str) -> String,
    deliveries: &mut Vec<Delivery>,
) {
    let number = 1 + random.next_below(5);
    let message = text(&format!(
        "WORKMAIL_0D_{}_{number}",
        owner.company_align.game_name()
    ));
    let Some(contact) = &owner.day_zero_contact else {
        return;
    };

    deliveries.push(Delivery {
        to: owner.colleague.address.clone(),
        other: contact.clone(),
        subject: "About the company".to_owned(),
        text: message.clone(),
        sending: true,
    });
    deliveries.push(Delivery {
        to: contact.clone(),
        other: owner.colleague.address.clone(),
        subject: "About the company".to_owned(),
        text: message,
        sending: false,
    });
}

/// Selects an item of `items` with `Random.Next(items.Count)`.
fn pick<T: Copy>(random: &mut Random, items: &[T]) -> Option<T> {
    (!items.is_empty()).then(|| items[random.next_below(items.len())])
}

/// `List<string>.Remove`: removes the first entry equal to `name`.
fn remove_first(names: &mut Vec<String>, name: &str) {
    if let Some(index) = names.iter().position(|candidate| candidate == name) {
        names.remove(index);
    }
}
