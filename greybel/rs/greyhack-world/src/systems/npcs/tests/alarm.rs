use greyhack_behaviour::NetworkKind;

use crate::AlarmFailure;
use crate::systems::npcs::jobs::tests::support::{public_ip, world_with};

#[tokio::test]
async fn should_validate_phone_alarm_times() {
    let (mut world, network) = world_with(NetworkKind::Supermarket).await;
    let address = public_ip(&world, network).await;

    assert_eq!(
        world
            .set_phone_alarm(&address, "nobody", 24, 0, false)
            .await
            .unwrap(),
        Some(Err(AlarmFailure::IncorrectTime))
    );
    assert_eq!(
        world
            .set_phone_alarm(&address, "nobody", 12, 0, false)
            .await
            .unwrap(),
        Some(Err(AlarmFailure::IncompatibleDevice))
    );
}
