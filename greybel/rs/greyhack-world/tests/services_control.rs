use greyhack_behaviour::{ROOT_USER, ServiceId};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_services::ServiceControlFailure;

#[tokio::test]
async fn should_reject_http_install_start_and_stop_on_a_router() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = world
        .modules
        .core
        .networks
        .topology
        .router_of(world.handle.player())
        .await
        .unwrap()
        .unwrap();
    let computer = router.id.computer();

    assert_eq!(
        world
            .modules
            .services
            .control
            .control_install_service(computer, ROOT_USER, ServiceId::Http)
            .await
            .unwrap(),
        Err(ServiceControlFailure::RouterHttp)
    );
    assert_eq!(
        world
            .modules
            .services
            .control
            .control_start_service(computer, ROOT_USER, ServiceId::Http)
            .await
            .unwrap(),
        Err(ServiceControlFailure::RouterHttp)
    );
    assert_eq!(
        world
            .modules
            .services
            .control
            .control_stop_service(computer, ROOT_USER, ServiceId::Http)
            .await
            .unwrap(),
        Err(ServiceControlFailure::RouterHttp)
    );
}
