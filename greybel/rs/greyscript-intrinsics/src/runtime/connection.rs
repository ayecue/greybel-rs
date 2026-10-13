//! Shared game-server connection for program operations, object handles, output routing, and end/throttle notifications.

use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;
use std::time::Duration;

use duplex_core::{Channel, Driver};
use greyhack_behaviour::META_MAIL_COOLDOWN;

use super::handle::{Handle, HandleTable};
use super::programs::ProgramTable;
use crate::protocol::{
    ObjectId, Program, ProgramChannel, ProgramId, ProgramRouter, ReleaseObjects,
};

/// Connection shared by a runtime's programs; clones share its channel and routing state.
#[derive(Clone)]
pub struct ServerConnection {
    channel: Rc<dyn ProgramChannel>,
    programs: Rc<ProgramTable>,
    handles: Rc<HandleTable>,
    /// Player's last MetaMail use on the program clock (`MailHelperServer.metaMailCooldownWatch`).
    last_mail_at: Rc<Cell<Option<Duration>>>,
}

impl ServerConnection {
    /// Creates a connection over `channel` and installs output and program-end routing.
    #[must_use]
    pub fn new<D: Driver>(channel: Channel<Program, D>) -> Self {
        let programs = Rc::new(ProgramTable::default());

        Rc::clone(&programs).attach(&channel);

        Self {
            channel: Rc::new(channel),
            programs,
            handles: Rc::default(),
            last_mail_at: Rc::default(),
        }
    }

    /// Applies the player's MetaMail cooldown at `now` (`MailHelperServer.UseMetaMailGlobalCooldown`).
    /// Returns false within [`META_MAIL_COOLDOWN`] of the previous use; otherwise restarts the cooldown and returns true.
    #[must_use]
    pub fn use_mail_cooldown(&self, now: Duration) -> bool {
        if self
            .last_mail_at
            .get()
            .is_some_and(|previous| now.saturating_sub(previous) < META_MAIL_COOLDOWN)
        {
            return false;
        }

        self.last_mail_at.set(Some(now));

        true
    }

    /// Flushes queued object releases and returns the operation channel.
    #[must_use]
    pub fn channel(&self) -> &Rc<dyn ProgramChannel> {
        let releases = self.handles.take_releases();

        if !releases.is_empty() {
            let _ = self.channel.send_release_objects(&ReleaseObjects {
                objects: releases.into(),
            });
        }

        &self.channel
    }

    /// Returns the live handle for server object `id`, or creates one if none exists.
    #[must_use]
    pub(crate) fn object(&self, id: ObjectId) -> Handle {
        self.handles.object(id)
    }

    /// Returns the program-end/throttle notification count used by [`ServerConnection::notice_after`].
    pub(crate) fn notice_count(&self) -> u64 {
        self.programs.notice_count()
    }

    /// Resolves once the notification count exceeds `count`; see [`ServerConnection::notice_count`].
    pub(crate) fn notice_after(&self, count: u64) -> impl Future<Output = ()> + 'static {
        self.programs.notice_after(count)
    }

    /// Removes and returns the longest server-requested pause for `programs`.
    pub(crate) fn take_pause(
        &self,
        programs: impl IntoIterator<Item = ProgramId>,
    ) -> Option<Duration> {
        self.programs.take_pause(programs)
    }

    /// Reports whether the game server ended `program`; subsequent operations fail.
    #[must_use]
    pub fn has_ended(&self, program: ProgramId) -> bool {
        self.programs.has_ended(program)
    }

    /// Removes `program`'s ended-state and pause tracking.
    pub fn forget(&self, program: ProgramId) {
        self.programs.forget(program);
    }
}
