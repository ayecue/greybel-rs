//! Default values and size limits for mail.

use std::time::Duration;

use crate::base::refusal::{Refusal, RuleResult};

/// Mail domain used when a computer's router has no domain (`Computer.ConfiguringServices`).
pub const DEFAULT_MAIL_DOMAIN: &str = "domain-default.com";

/// Returns the mail address of an NPC with account `name` at `domain` that is the `attempt`-th the
/// world tries: the account's own name first, then the name numbered from 1 for as long as the
/// address is taken (`NpcInfo.NpcInfo`).
#[must_use]
pub fn npc_mail_address(name: &str, domain: &str, attempt: u32) -> String {
    if attempt == 0 {
        format!("{name}@{domain}")
    } else {
        format!("{name}{attempt}@{domain}")
    }
}

/// Maximum address length accepted by `send`, in string positions.
pub const MAX_MAIL_ADDRESS: usize = 64;
/// Maximum subject length accepted by `send`, in string positions.
pub const MAX_MAIL_SUBJECT: usize = 128;
/// Maximum message length accepted by `send`, in string positions.
pub const MAX_MAIL_MESSAGE: usize = 2500;
/// Number of body characters included in an inbox summary.
pub const MAIL_PREVIEW: usize = 125;
/// Minimum time between uses of `MetaMail` methods, for both the program's stopwatch (`HelperScript.UseMetaMailCooldownOrError`)
/// and the player's (`MailHelperServer.UseMetaMailGlobalCooldown`).
pub const META_MAIL_COOLDOWN: Duration = Duration::from_secs(2);

/// Validates mail passed to `send`: the address needs a name before `@`, and address, subject, and
/// message lengths use the game's UTF-16 limits.
pub fn check_mail(to: &str, subject: &str, message: &str) -> RuleResult {
    let length = |text: &str| text.encode_utf16().count();
    let has_name = to.split_once('@').is_some_and(|(name, _)| !name.is_empty());

    if !has_name || length(to) > MAX_MAIL_ADDRESS {
        Err(Refusal::InvalidMailAddress)
    } else if length(subject) > MAX_MAIL_SUBJECT {
        Err(Refusal::MailSubjectTooLong)
    } else if length(message) > MAX_MAIL_MESSAGE {
        Err(Refusal::MailMessageTooLong)
    } else {
        Ok(())
    }
}
