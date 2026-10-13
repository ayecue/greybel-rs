use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use super::{Endpoint, TransportState};
use crate::driver::Driver;
use crate::hub::ProcessResult;
use crate::keys::{EndpointId, Work};

const PAUSE_ABOVE: usize = 4096;
const RESUME_BELOW: usize = 1024;

#[derive(Default)]
pub(super) struct WorkQueue {
    queue: RefCell<VecDeque<(EndpointId, Work)>>,
    in_hub_queue: Cell<bool>,
    paused: Cell<bool>,
}

impl<D: Driver> TransportState<D> {
    pub(crate) fn enqueue(&self, endpoint: EndpointId, work: Work) {
        let queue_len = {
            let mut queue = self.work_queue.queue.borrow_mut();

            queue.push_back((endpoint, work));

            queue.len()
        };

        if queue_len > PAUSE_ABOVE && !self.work_queue.paused.replace(true) {
            self.driver.pause();
        }

        let joins = !self.work_queue.in_hub_queue.replace(true);

        self.hub.enqueue(self.slot(), joins);
    }

    pub(super) fn clear_work_queue(&self) {
        let count = std::mem::take(&mut *self.work_queue.queue.borrow_mut()).len();

        self.hub.release(count);
    }

    pub(super) fn process_next(&self, limit: usize) -> ProcessResult {
        let mut count = 0;
        let mut current: Option<(EndpointId, Option<Rc<dyn Endpoint>>)> = None;

        while count < limit {
            let next = self.work_queue.queue.borrow_mut().pop_front();
            let Some((endpoint, work)) = next else {
                break;
            };

            if current
                .as_ref()
                .is_none_or(|(current_id, _)| *current_id != endpoint)
            {
                current = Some((endpoint, self.endpoint(endpoint)));
            }

            if let Some((_, Some(hit))) = &current {
                hit.process(work);
            }

            count += 1;
        }

        let queue_len = self.work_queue.queue.borrow().len();

        if queue_len < RESUME_BELOW && self.work_queue.paused.replace(false) {
            self.driver.resume();
        }

        let more = queue_len > 0;

        self.work_queue.in_hub_queue.set(more);

        ProcessResult {
            count,
            has_next: more,
        }
    }
}
