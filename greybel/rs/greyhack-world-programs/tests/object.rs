use greyhack_world_programs::protocol::{ObjectId, ScriptFailure};
use greyhack_world_programs::{ObjectTable, RouteError};
use support::{HandleId, Other};

use crate::support;

#[test]
fn should_reuse_a_typed_objects_identity() {
    let mut objects = ObjectTable::default();
    let first = objects.insert(HandleId(1));

    assert_eq!(objects.insert(HandleId(1)), first);
    assert_eq!(objects.get::<HandleId, ()>(first).unwrap(), &HandleId(1));
    assert_eq!(objects.len(), 1);
}

#[test]
fn should_reject_unknown_objects_and_wrong_handle_types() {
    let mut objects = ObjectTable::default();
    let id = objects.insert(HandleId(1));

    assert!(matches!(
        objects.get::<Other, ()>(id),
        Err(RouteError::UnknownObject)
    ));
    assert!(matches!(
        objects.get::<HandleId, ()>(ObjectId { id: 99 }),
        Err(RouteError::UnknownObject)
    ));
}

#[test]
fn should_forget_released_objects_and_their_identity() {
    let mut objects = ObjectTable::default();
    let first = objects.insert(HandleId(1));

    objects.release(&[first]);

    assert!(objects.is_empty());
    assert!(matches!(
        objects.get::<HandleId, ()>(first),
        Err(RouteError::UnknownObject)
    ));
    assert_ne!(objects.insert(HandleId(1)), first);
}

#[test]
fn should_interrupt_objects_on_their_rebooted_computer() {
    let mut objects = ObjectTable::default();
    let id = objects.insert(HandleId(1));

    objects.interrupt(1);

    assert!(matches!(
        objects.get::<HandleId, ()>(id),
        Err(RouteError::Script(ScriptFailure::ConnectionInterrupted))
    ));

    objects.release(&[id]);

    assert!(matches!(
        objects.get::<HandleId, ()>(id),
        Err(RouteError::UnknownObject)
    ));
}
