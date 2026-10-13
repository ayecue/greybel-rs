use greyhack_behaviour::{ROOT_USER, ServiceId};

use crate::{PlayerOptions, ServiceControlFailure, World};

#[tokio::test]
async fn should_reject_http_install_start_and_stop_on_a_router() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = world.router_of(world.player()).await.unwrap().unwrap();
    let computer = router.id.computer();

    assert_eq!(
        world
            .control_install_service(computer, ROOT_USER, ServiceId::Http)
            .await
            .unwrap(),
        Err(ServiceControlFailure::RouterHttp)
    );
    assert_eq!(
        world
            .control_start_service(computer, ROOT_USER, ServiceId::Http)
            .await
            .unwrap(),
        Err(ServiceControlFailure::RouterHttp)
    );
    assert_eq!(
        world
            .control_stop_service(computer, ROOT_USER, ServiceId::Http)
            .await
            .unwrap(),
        Err(ServiceControlFailure::RouterHttp)
    );
}
