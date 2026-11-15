use greyhack_world_programs::protocol::Credential;

use super::IntoProtocol;

impl IntoProtocol for greyhack_behaviour::Credential {
    type Output = Credential;

    fn into_protocol(self) -> Self::Output {
        match self {
            greyhack_behaviour::Credential::UserName => Credential::UserName,
            greyhack_behaviour::Credential::Password => Credential::Password,
            greyhack_behaviour::Credential::GroupName => Credential::GroupName,
        }
    }
}
