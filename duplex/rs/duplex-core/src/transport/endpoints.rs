use std::cell::RefCell;
use std::rc::Rc;

use slotmap::SlotMap;

use super::Endpoint;
use crate::definition::Role;
use crate::keys::EndpointId;

struct Registration {
    channel: u32,
    role: Role,
    endpoint: Rc<dyn Endpoint>,
}

#[derive(Default)]
pub(super) struct EndpointTable(RefCell<SlotMap<EndpointId, Registration>>);

impl EndpointTable {
    pub(super) fn attach<E: Endpoint + 'static>(
        &self,
        channel: u32,
        role: Role,
        endpoint: impl FnOnce(EndpointId) -> Rc<E>,
    ) -> Rc<E> {
        assert!(
            self.find(channel, role).is_none(),
            "the transport already has a {role:?} of the channel {channel}"
        );

        let mut new_endpoint = None;

        self.0.borrow_mut().insert_with_key(|handle| {
            let endpoint = endpoint(handle);

            new_endpoint = Some(Rc::clone(&endpoint));

            Registration {
                channel,
                role,
                endpoint,
            }
        });

        new_endpoint.expect("the endpoint was just attached")
    }

    pub(super) fn detach(&self, handle: EndpointId) {
        self.0.borrow_mut().remove(handle);
    }

    pub(super) fn find(&self, channel: u32, role: Role) -> Option<Rc<dyn Endpoint>> {
        self.0
            .borrow()
            .values()
            .find(|entry| (entry.channel, entry.role) == (channel, role))
            .map(|entry| Rc::clone(&entry.endpoint))
    }

    pub(super) fn get(&self, handle: EndpointId) -> Option<Rc<dyn Endpoint>> {
        self.0
            .borrow()
            .get(handle)
            .map(|entry| Rc::clone(&entry.endpoint))
    }

    pub(super) fn all(&self) -> Vec<Rc<dyn Endpoint>> {
        self.0
            .borrow()
            .values()
            .map(|entry| Rc::clone(&entry.endpoint))
            .collect()
    }
}
