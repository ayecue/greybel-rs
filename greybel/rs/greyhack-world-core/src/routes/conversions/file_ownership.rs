use greyhack_world_programs::protocol::FileOwnership;

use super::IntoProtocol;

impl IntoProtocol for greyhack_behaviour::FileOwnership {
    type Output = FileOwnership;

    fn into_protocol(self) -> Self::Output {
        match self {
            greyhack_behaviour::FileOwnership::Owner => FileOwnership::Owner,
            greyhack_behaviour::FileOwnership::Group => FileOwnership::Group,
        }
    }
}
