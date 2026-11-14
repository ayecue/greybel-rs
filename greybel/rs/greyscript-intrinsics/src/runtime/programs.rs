//! Per-program ended flags and server-requested pauses.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::rc::Rc;
use std::time::Duration;

use duplex_core::NextNotification;
use greybel_interpreter::Stdout;
use tokio::sync::Notify;

use crate::game::print_line;
use crate::protocol::{
    ProgramEnd, ProgramId, ProgramPrint, ProgramPrintRef, ProgramRouter, ProgramThrottle,
};

/// Per-program ended flags and pending pauses.
#[derive(Default)]
pub(super) struct ProgramTable {
    ends: RefCell<HashSet<u32>>,
    /// Terminal output of each active program.
    outputs: RefCell<HashMap<u32, Rc<dyn Stdout>>>,
    /// Pause duration in milliseconds for each throttled program tree.
    pauses: RefCell<HashMap<u32, u32>>,
    /// Count of received program-end and throttle notifications.
    notice_count: Cell<u64>,
    /// Wakes tasks waiting for additional program-end or throttle notifications.
    notice_notify: Notify,
}

impl ProgramTable {
    fn count_notice(&self) {
        self.notice_count.set(self.notice_count.get() + 1);
        self.notice_notify.notify_waiters();
    }

    /// Returns the program-end/throttle notification count used by [`ProgramTable::notice_after`].
    pub(super) fn notice_count(&self) -> u64 {
        self.notice_count.get()
    }

    /// Resolves once the notification count exceeds `count`; see [`ProgramTable::notice_count`].
    pub(super) fn notice_after(self: &Rc<Self>, count: u64) -> impl Future<Output = ()> + 'static {
        let programs = Rc::clone(self);

        async move {
            loop {
                let notification = programs.notice_notify.notified();
                let mut notification = std::pin::pin!(notification);

                notification.as_mut().enable();

                if programs.notice_count.get() > count {
                    return;
                }

                notification.await;
            }
        }
    }

    /// Removes and returns the longest server-requested pause for `programs`.
    pub(super) fn take_pause(
        &self,
        programs: impl IntoIterator<Item = ProgramId>,
    ) -> Option<Duration> {
        let mut pauses = self.pauses.borrow_mut();

        programs
            .into_iter()
            .filter_map(|program| pauses.remove(&program.id))
            .max()
            .map(|milliseconds| Duration::from_millis(u64::from(milliseconds)))
    }

    /// Reports whether the game server ended `program`.
    pub(super) fn has_ended(&self, program: ProgramId) -> bool {
        self.ends.borrow().contains(&program.id)
    }

    /// Routes later output of `program` to `stdout`.
    pub(super) fn register_output(&self, program: ProgramId, stdout: Rc<dyn Stdout>) {
        self.outputs.borrow_mut().insert(program.id, stdout);
    }

    /// Removes `program`'s output, ended-state, and pause tracking.
    pub(super) fn forget(&self, program: ProgramId) {
        self.ends.borrow_mut().remove(&program.id);
        self.outputs.borrow_mut().remove(&program.id);
        self.pauses.borrow_mut().remove(&program.id);
    }
}

impl ProgramRouter for Rc<ProgramTable> {
    fn on_program_end(&self, message: ProgramEnd, _next: NextNotification<ProgramEnd>) {
        self.ends.borrow_mut().insert(message.program.id);
        self.count_notice();
    }

    fn on_program_print(
        &self,
        message: ProgramPrintRef<'_>,
        _next: NextNotification<ProgramPrint>,
    ) {
        let output = self.outputs.borrow().get(&message.program.id).cloned();

        if let Some(output) = output {
            print_line(output.as_ref(), &message.line.into_owned());
        }
    }

    fn on_program_throttle(
        &self,
        message: ProgramThrottle,
        _next: NextNotification<ProgramThrottle>,
    ) {
        let mut pauses = self.pauses.borrow_mut();
        let pause = pauses.entry(message.program.id).or_default();

        *pause = (*pause).max(message.milliseconds);
        drop(pauses);
        self.count_notice();
    }
}
