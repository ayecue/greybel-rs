use greyhack_behaviour::Refusal;

use crate::game::client::failure::refusal_text;

/// Returns the text `refusal` produces for a returned (not raised) refusal.
pub fn text(refusal: &Refusal) -> String {
    refusal_text("test", refusal).expect("text refusal")
}
