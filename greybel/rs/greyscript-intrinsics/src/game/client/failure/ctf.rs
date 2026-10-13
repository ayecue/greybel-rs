//! Grey Hack failure text for CTF accounts and event ratings.

use crate::protocol::CtfFailure;

/// Formats `PlayerIntrinsics.reset_ctf_password` and CTF-event rating failures.
pub(super) const fn ctf_text(failure: CtfFailure) -> &'static str {
    match failure {
        CtfFailure::NoAccount => "reset_ctf_password: you do not have a registered CTF account",
        CtfFailure::InvalidScore => "Invalid score",
        CtfFailure::UnknownEvent => "Event not found",
        CtfFailure::Incomplete => "You have to complete the mission to be able to rate it",
        CtfFailure::RepeatScore => "You have already rated this mission",
    }
}
