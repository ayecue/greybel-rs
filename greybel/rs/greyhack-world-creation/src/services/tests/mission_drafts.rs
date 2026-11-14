#[path = "support/mission_drafts.rs"]
pub(crate) mod support;

use support::{assignment, draft, npc};

use super::replaced_account;

/// The mission NPC takes the account named like it, or the one the replacement draw picks among the non-root
/// accounts, and none on a computer that has only root (`NetworkLan.UpdateNpcMissionNames`).
#[test]
fn should_pick_the_account_the_mission_npc_replaces() {
    let draft_of = draft(&["Aaa", "Bbb", "Ccc"]);

    assert_eq!(
        replaced_account(&draft_of, &npc("Bbb"), &assignment(Some(0.0))),
        Some("Bbb".to_owned())
    );
    assert_eq!(
        replaced_account(&draft_of, &npc("Zzz"), &assignment(Some(0.0))),
        Some("Aaa".to_owned())
    );
    assert_eq!(
        replaced_account(&draft_of, &npc("Zzz"), &assignment(Some(0.5))),
        Some("Bbb".to_owned())
    );
    assert_eq!(
        replaced_account(&draft_of, &npc("Zzz"), &assignment(Some(0.999))),
        Some("Ccc".to_owned())
    );
    assert_eq!(
        replaced_account(&draft(&[]), &npc("Zzz"), &assignment(Some(0.5))),
        None
    );
}
