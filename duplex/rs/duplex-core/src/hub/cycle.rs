use std::rc::Rc;
use std::time::Duration;

use super::{HubState, ProcessResult, Schedulable};
use crate::keys::TransportId;

const CYCLE_WORK: usize = 1024;
const CYCLE_TIME: Duration = Duration::from_millis(1);
const CLOCK_EVERY: usize = 64;
const TURN: usize = 16;

impl HubState {
    pub(crate) fn enqueue(&self, transport: TransportId, joins: bool) {
        self.incoming_count.set(self.incoming_count.get() + 1);
        if joins {
            self.queue.borrow_mut().push_back(transport);
        }

        if self.depth.get() == 0 {
            self.schedule();
        }
    }

    pub(crate) fn release(&self, count: usize) {
        self.incoming_count
            .set(self.incoming_count.get().saturating_sub(count));
    }

    pub(crate) fn process_read_batch(&self) {
        let count = self.incoming_count.get();

        if self.running.get() || count == 0 {
            return;
        }

        if count <= CYCLE_WORK {
            self.process();
        } else {
            self.schedule();
        }
    }

    pub(crate) fn enter_cycle(&self) {
        self.depth.set(self.depth.get() + 1);
    }

    pub(crate) fn exit_cycle(&self) {
        let depth = self.depth.get() - 1;

        self.depth.set(depth);
        if depth == 0 && !self.running.get() && self.incoming_count.get() > 0 {
            self.schedule();
        }
    }

    fn schedule(&self) {
        if self.scheduled.get() {
            return;
        }

        let front = self.queue.borrow().front().copied();
        let Some(transport) = front.and_then(|front| self.transport(front)) else {
            return;
        };

        self.scheduled.set(true);

        let this = self.weak();

        transport.defer(Box::new(move || {
            if let Some(hub) = this.upgrade() {
                hub.scheduled.set(false);
                hub.process();
            }
        }));
    }

    fn process(&self) {
        if self.running.replace(true) {
            return;
        }

        self.enter_cycle();

        let mut start_time = None;
        let mut completed_items = 0;
        let mut current: Option<(TransportId, Rc<dyn Schedulable>)> = None;

        loop {
            let next = self.queue.borrow_mut().pop_front();
            let Some(handle) = next else {
                break;
            };
            let transport = match &current {
                Some((current_handle, transport)) if *current_handle == handle => {
                    Rc::clone(transport)
                }
                _ => {
                    let Some(transport) = self.transport(handle) else {
                        continue;
                    };

                    current = Some((handle, Rc::clone(&transport)));

                    transport
                }
            };
            let start_time = *start_time.get_or_insert_with(|| transport.now());
            let ProcessResult { count, has_next } =
                transport.process_next(TURN.min(CYCLE_WORK - completed_items));

            self.release(count);
            if has_next {
                self.queue.borrow_mut().push_back(handle);
            }

            let before = completed_items;

            completed_items += count;
            if completed_items >= CYCLE_WORK
                || (completed_items / CLOCK_EVERY > before / CLOCK_EVERY
                    && transport.now().saturating_sub(start_time) >= CYCLE_TIME)
            {
                break;
            }
        }

        self.running.set(false);
        if self.incoming_count.get() > 0 {
            self.schedule();
        }

        self.exit_cycle();
    }
}
