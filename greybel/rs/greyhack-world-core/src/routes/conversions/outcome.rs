use greyhack_world_programs::protocol::Outcome;

use super::IntoProtocol;

impl IntoProtocol for bool {
    type Output = Outcome;

    fn into_protocol(self) -> Self::Output {
        if self { Outcome::True } else { Outcome::False }
    }
}
