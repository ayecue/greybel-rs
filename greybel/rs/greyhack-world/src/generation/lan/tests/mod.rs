#[path = "support/mod.rs"]
mod support;

use greyhack_behaviour::DeviceKind;
use support::plans;

#[test]
fn should_give_a_network_with_a_smart_appliance_a_camera() {
    let mut appliances = 0;

    for plan in plans() {
        let kinds: Vec<DeviceKind> = plan
            .devices
            .iter()
            .flat_map(|device| device.computers.iter().map(|computer| computer.kind))
            .collect();

        if kinds
            .iter()
            .any(|kind| matches!(kind, DeviceKind::Fridge | DeviceKind::Microwave))
        {
            appliances += 1;

            assert!(kinds.contains(&DeviceKind::Camera), "{kinds:?}");
        }
    }

    assert!(appliances > 0);
}
