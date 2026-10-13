use greybel_test_support::future::block_on;
use greyhack_behaviour::{FIRST_VERSION, FileData, LibraryKind};
use greyhack_world::{Library, PlayerOptions, SmartApplianceSettings, World};

use super::add_library;

/// A world whose player computer is a smart appliance with its override settings enabled, as an exploit leaves it.
pub fn world_with_appliance() -> World {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let player = world.player();
    let settings = SmartApplianceSettings {
        override_enabled: true,
        ..SmartApplianceSettings::new("ACME/FR1", 600, 20)
    };

    block_on(world.save_smart(player, Some(&settings))).expect("the smart appliance is saved");
    add_library(
        &mut world,
        player,
        "libsmartappliance.so",
        FileData::library(0),
        Some(Library {
            kind: LibraryKind::SmartAppliance,
            version: FIRST_VERSION.to_owned(),
            zones: Vec::new(),
        }),
    );

    world
}
