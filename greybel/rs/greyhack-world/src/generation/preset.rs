//! Player computer and router presets.

use greyhack_behaviour::{ROOT_USER, START_DATE};
use greyhack_world_core::model::{STANDARD_DISK_MB, STANDARD_RAM_MB};
use greyhack_world_core::{Hardware, MinerHardware, NetCard, Part, User};
use greyhack_world_creation::{
    NetworkLayout, add_player_libraries, build_filesystem, system_processes,
};
use greyhack_world_libraries::ComputerDraft;

/// Player identity and starting computer settings.
#[derive(Debug, Clone)]
pub struct PlayerOptions {
    pub user: String,
    pub password: String,
    pub computer_name: String,
    pub local_ip: String,
    pub public_ip: String,
    /// Player mail account. `None` for an unregistered new-game player.
    pub mail_address: Option<String>,
    /// Mail password if different from the computer account password.
    pub mail_password: Option<String>,
    /// Player bank account. `None` for a new-game player without one.
    pub bank_number: Option<String>,
    /// Player bank balance.
    pub bank_money: f64,
    /// Additional player `/lib` libraries beyond `crypto.so`, the only new-game library. Defaults
    /// include libraries needed to test sandbox exploits immediately.
    pub libraries: Vec<String>,
    pub date: String,
    /// Explicit player network layout containing the player computer. Replaces the default
    /// one-computer network.
    pub network: Option<NetworkLayout>,
}

impl Default for PlayerOptions {
    fn default() -> Self {
        Self {
            user: "test".to_owned(),
            password: "test".to_owned(),
            computer_name: "test".to_owned(),
            local_ip: "192.168.0.2".to_owned(),
            public_ip: "1.1.1.1".to_owned(),
            mail_address: Some("test@test.com".to_owned()),
            mail_password: None,
            bank_number: Some("test".to_owned()),
            bank_money: 3000.0,
            libraries: ["metaxploit.so", "blockchain.so", "libssh.so"]
                .map(str::to_owned)
                .to_vec(),
            date: START_DATE.to_owned(),
            network: None,
        }
    }
}

/// Builds the player computer before router attachment.
pub(super) fn player_computer(options: &PlayerOptions) -> ComputerDraft {
    let mut computer = ComputerDraft::new(&options.computer_name, &options.local_ip);

    computer.is_player = true;

    computer.hardware = Hardware::standard(
        STANDARD_RAM_MB,
        STANDARD_DISK_MB,
        &MinerHardware::default(),
        vec![NetCard {
            part: Part::new("Atheros AR9271"),
            active: true,
            ..NetCard::new(true, true)
        }],
    );

    let mut player = User::new(&options.user, &options.password);

    // `PlayerComputer.CreatePlayer` adds the player user to the root group.
    player.add_group(ROOT_USER);
    computer.users = vec![User::new(ROOT_USER, &options.password), player];
    computer.processes = system_processes();
    build_filesystem(&mut computer);
    add_player_libraries(&mut computer, &options.user, &options.libraries);

    computer
}
