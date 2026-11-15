use greyhack_world_programs::protocol::Side;

use super::IntoProtocol;

impl IntoProtocol for greyhack_behaviour::Side {
    type Output = Side;

    fn into_protocol(self) -> Self::Output {
        match self {
            Self::Buy => Side::Buy,
            Self::Sell => Side::Sell,
        }
    }
}
