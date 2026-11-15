use std::rc::Rc;

use greyhack_world_modules::Registry;

use super::support::registry::{Call, FirstService, Provider, SecondService};

#[test]
fn should_resolve_registered_services_on_first_use() {
    let registry = Rc::new(Registry::new(7_u32));
    let service = registry.get::<String>();
    let call = registry.get::<dyn Call>();

    assert_eq!(registry.missing_services().len(), 2);

    registry.register_service(Rc::new("service".to_owned()));
    registry.register_service::<dyn Call>(Rc::new(Provider));

    assert_eq!(*registry.world::<u32>(), 7);
    assert_eq!(service.as_str(), "service");
    assert_eq!(call.call(), 42);
    assert!(registry.missing_services().is_empty());
}

#[test]
fn should_name_an_unregistered_dependency_on_first_use() {
    let registry = Rc::new(Registry::new(()));
    let missing = registry.get::<dyn Call>();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| missing.call()));
    let error = panic.expect_err("an unregistered dependency panics");
    let text = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap();

    assert!(text.contains("Call"), "{text}");
    assert!(text.contains("is not registered"), "{text}");
    assert_eq!(registry.missing_services().len(), 1);
}

#[test]
fn should_allow_two_services_to_depend_on_each_other() {
    let registry = Rc::new(Registry::new(()));
    let first = Rc::new(FirstService {
        other: registry.get(),
    });
    let second = Rc::new(SecondService {
        other: registry.get(),
    });

    registry.register_service(first.clone());
    registry.register_service(second.clone());

    assert_eq!(first.call(), 42);
    assert_eq!(second.call(), 41);
    assert!(registry.missing_services().is_empty());
}
