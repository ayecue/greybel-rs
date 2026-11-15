use greyhack_world_programs::protocol::NameKind;

use super::IntoProtocol;

impl IntoProtocol for greyhack_behaviour::NameKind {
    type Output = NameKind;

    fn into_protocol(self) -> Self::Output {
        match self {
            greyhack_behaviour::NameKind::File => NameKind::File,
            greyhack_behaviour::NameKind::Folder => NameKind::Folder,
            greyhack_behaviour::NameKind::New => NameKind::New,
        }
    }
}
