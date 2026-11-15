use std::cell::RefCell;
use std::rc::Rc;

use greyhack_world_effects::{EventQueue, WorkCompletion, WorkQueue};

use super::support::effects::{First, Second};

#[test]
fn should_dispatch_events_in_publish_order_across_types() {
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
    events.dispatch();

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
    events.dispatch();

    assert_eq!(*seen.borrow(), [9, 0]);
}
