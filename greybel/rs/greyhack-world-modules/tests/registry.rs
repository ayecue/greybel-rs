use std::cell::RefCell;
use std::rc::Rc;

use greyhack_world_modules::{EventQueue, Registry, WorkCompletion, WorkQueue};

use super::support::registry::{Call, First, FirstService, Provider, Second, SecondService};

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

#[test]
fn should_deliver_events_in_publish_order_across_types() {
    let events = EventQueue::default();
    let order = Rc::new(RefCell::new(Vec::new()));

    events.on({
        let order = order.clone();

        move |event: &First| order.borrow_mut().push(event.0)
    });
    events.on({
        let order = order.clone();

        move |event: &Second| order.borrow_mut().push(event.0)
    });

    events.publish(Second(2));
    events.publish(First(1));
    events.publish(Second(3));
    events.deliver();

    assert_eq!(*order.borrow(), [2, 1, 3]);
    assert!(events.is_empty());
}

#[test]
fn should_finish_and_cancel_typed_work_in_event_order() {
    let events = Rc::new(EventQueue::default());
    let works = WorkQueue::new(events.clone());
    let finished = works.start::<u32>();
    let cancelled = works.start::<u32>();
    let seen = Rc::new(RefCell::new(Vec::new()));

    events.on({
        let seen = seen.clone();

        move |end: &WorkCompletion| {
            let value = match end.id {
                id if id == finished.id() => end.take::<u32>().unwrap().unwrap(),
                id if id == cancelled.id() => {
                    assert!(end.take::<u32>().unwrap().is_err());

                    0
                }
                _ => unreachable!(),
            };

            seen.borrow_mut().push(value);
        }
    });

    works.finish(finished.id(), 9_u32);
    works.cancel(cancelled.id());
    events.deliver();

    assert_eq!(*seen.borrow(), [9, 0]);
}
