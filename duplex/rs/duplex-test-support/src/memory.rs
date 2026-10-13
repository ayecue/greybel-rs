use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::pin;
use std::rc::Rc;
use std::task::Poll;
use std::time::Duration;

use duplex_core::{
    Channel, ChannelOptions, Driver, OutboundBatch, Peer, Role, Transport, WeakTransport,
};

use super::frames::{AliasTable, Slice};
use super::future::poll;
use super::scheduler::ManualScheduler;

pub struct Memory {
    queue: MemoryQueue,
    to: usize,
}

#[derive(Clone, Default)]
pub struct MemoryQueue(Rc<Queue>);

#[derive(Default)]
struct Queue {
    batches: RefCell<VecDeque<PostedBatch>>,
    aliases: RefCell<[AliasTable; 2]>,
    scheduler: ManualScheduler,
    peers: RefCell<[Option<AttachedPeer>; 2]>,
    sealed: std::cell::Cell<bool>,
    paused: std::cell::Cell<[bool; 2]>,
}

struct PostedBatch {
    to: usize,
    batch: Rc<[u8]>,
    slices: Vec<Slice>,
}

struct AttachedPeer {
    role: Role,
    transport: WeakTransport<Memory>,
}

impl Memory {
    pub fn pair<P>() -> (Channel<P, Self>, Channel<P::Remote, Self>, MemoryQueue)
    where
        P: Peer<OpenPayload = ()>,
        P::Remote: Peer<OpenPayload = ()>,
    {
        Self::pair_with_options::<P>(ChannelOptions::default())
    }

    pub fn pair_with_options<P>(
        options: ChannelOptions,
    ) -> (Channel<P, Self>, Channel<P::Remote, Self>, MemoryQueue)
    where
        P: Peer<OpenPayload = ()>,
        P::Remote: Peer<OpenPayload = ()>,
    {
        let queue = MemoryQueue::default();
        let first = Channel::new(&queue.transport(0, P::ROLE), options);
        let second = Channel::new(&queue.transport(1, <P::Remote as Peer>::ROLE), options);

        (first, second, queue)
    }

    /// Creates default channels with [`Memory::pair`] and delivers initial handshake batches.
    pub fn connected_pair<P>() -> (Channel<P, Self>, Channel<P::Remote, Self>, MemoryQueue)
    where
        P: Peer<OpenPayload = ()>,
        P::Remote: Peer<OpenPayload = ()>,
    {
        Self::connected_pair_with_options::<P>(ChannelOptions::default())
    }

    /// Creates channels with [`Memory::pair_with_options`] and delivers initial handshake batches.
    pub fn connected_pair_with_options<P>(
        options: ChannelOptions,
    ) -> (Channel<P, Self>, Channel<P::Remote, Self>, MemoryQueue)
    where
        P: Peer<OpenPayload = ()>,
        P::Remote: Peer<OpenPayload = ()>,
    {
        let (first, second, queue) = Self::pair_with_options::<P>(options);

        queue.deliver();

        (first, second, queue)
    }

    #[must_use]
    pub fn transports() -> (Transport<Self>, Transport<Self>, MemoryQueue) {
        let queue = MemoryQueue::default();
        let client = queue.transport(0, Role::Client);
        let provider = queue.transport(1, Role::Provider);

        (client, provider, queue)
    }

    #[cfg(feature = "security")]
    #[must_use]
    pub fn transports_with_security(
        client: duplex_core::Security,
        provider: duplex_core::Security,
    ) -> (Transport<Self>, Transport<Self>, MemoryQueue) {
        let queue = MemoryQueue::default();

        queue.0.sealed.set(true);

        let provider = queue.transport_with_security(1, Role::Provider, provider);
        let client = queue.transport_with_security(0, Role::Client, client);

        (client, provider, queue)
    }

    /// Creates client `C` and provider `S` channels with separate options and a shared memory queue.
    pub fn between_with<C, S>(
        client: ChannelOptions,
        provider: ChannelOptions,
    ) -> (Channel<C, Self>, Channel<S, Self>, MemoryQueue)
    where
        C: Peer<OpenPayload = ()>,
        S: Peer<OpenPayload = ()>,
    {
        let queue = MemoryQueue::default();
        let client = Channel::new(&queue.transport(0, Role::Client), client);
        let provider = Channel::new(&queue.transport(1, Role::Provider), provider);

        (client, provider, queue)
    }

    pub fn open_pair<P>(
        options: ChannelOptions,
        open_payload: &P::OpenPayload,
    ) -> (Channel<P, Self>, Channel<P::Remote, Self>, MemoryQueue)
    where
        P::Remote: Peer<OpenPayload = ()>,
        P: Peer,
    {
        let queue = MemoryQueue::default();
        let client = Channel::open(&queue.transport(0, Role::Client), options, open_payload)
            .expect("writes what the client opens with");
        let provider = Channel::new(&queue.transport(1, Role::Provider), options);

        (client, provider, queue)
    }
}

impl MemoryQueue {
    pub fn deliver(&self) -> usize {
        let mut steps = 0;

        while self.step() {
            steps += 1;
        }

        steps
    }

    /// Polls `future` to completion and delivers queued batches after each pending poll. Panics if the future is pending
    /// with no queued batches.
    pub fn drive<F: Future>(&self, future: F) -> F::Output {
        let mut future = pin!(future);

        loop {
            if let Poll::Ready(output) = poll(&mut future) {
                return output;
            }

            assert!(self.deliver() > 0, "the future waits for no batch");
        }
    }

    /// Polls `future` to completion, calling `between` and delivering batches after each pending poll. Panics after
    /// 100,000 incomplete rounds.
    pub fn drive_with<F: Future>(&self, future: F, mut between: impl FnMut()) -> F::Output {
        const ROUNDS: usize = 100_000;
        let mut future = pin!(future);

        for _ in 0..ROUNDS {
            if let Poll::Ready(output) = poll(&mut future) {
                return output;
            }

            between();
            self.deliver();
        }

        panic!("the future still waits after {ROUNDS} rounds");
    }

    pub fn step(&self) -> bool {
        self.step_bytes().is_some()
    }

    pub fn deliver_bytes(&self) -> Vec<Vec<u8>> {
        std::iter::from_fn(|| self.step_bytes().map(|batch| batch.to_vec())).collect()
    }

    pub fn seal(&self) {
        self.0.sealed.set(true);
    }

    fn step_bytes(&self) -> Option<Rc<[u8]>> {
        self.run_tasks();

        let PostedBatch { to, batch, .. } = self.0.batches.borrow_mut().pop_front()?;
        let transport = self.0.peers.borrow()[to]
            .as_ref()
            .map(|peer| peer.transport.clone());

        if let Some(transport) = transport {
            transport.receive(Rc::clone(&batch));
        }

        Some(batch)
    }

    pub fn run_tasks(&self) {
        while self.run_task() {}
    }

    /// Runs the oldest deferred task if available and returns whether one ran.
    pub fn run_task(&self) -> bool {
        self.0.scheduler.run_task()
    }

    pub fn post(&self, role: Role, batch: Vec<u8>) {
        let to = self
            .0
            .peers
            .borrow()
            .iter()
            .position(|peer| peer.as_ref().is_some_and(|peer| peer.role == role));

        if let Some(to) = to {
            self.push(to, batch.into());
        }
    }

    #[must_use]
    pub fn queued_batches(&self) -> Vec<Vec<u8>> {
        self.run_tasks();

        self.0
            .batches
            .borrow()
            .iter()
            .map(|entry| entry.batch.to_vec())
            .collect()
    }

    #[must_use]
    pub fn queued_slices(&self) -> Vec<Slice> {
        self.run_tasks();

        self.0
            .batches
            .borrow()
            .iter()
            .flat_map(|entry| entry.slices.iter().cloned())
            .collect()
    }

    pub fn take(&self) -> Vec<Vec<u8>> {
        self.run_tasks();

        self.take_batches()
    }

    pub fn advance(&self, by: Duration) {
        self.0.scheduler.advance(by);
    }

    /// Returns whether the `role` transport has paused driver reads.
    #[must_use]
    pub fn is_paused(&self, role: Role) -> bool {
        self.0
            .peers
            .borrow()
            .iter()
            .position(|peer| peer.as_ref().is_some_and(|peer| peer.role == role))
            .is_some_and(|index| self.0.paused.get()[index])
    }

    fn set_paused(&self, index: usize, paused: bool) {
        let mut all = self.0.paused.get();

        all[index] = paused;
        self.0.paused.set(all);
    }

    #[must_use]
    pub fn timers(&self) -> Vec<Duration> {
        self.0.scheduler.intervals()
    }

    pub fn take_batches(&self) -> Vec<Vec<u8>> {
        self.0
            .batches
            .borrow_mut()
            .drain(..)
            .map(|entry| entry.batch.to_vec())
            .collect()
    }

    fn transport(&self, index: usize, role: Role) -> Transport<Memory> {
        let transport = Transport::new(|_| Memory {
            queue: self.clone(),
            to: 1 - index,
        });

        self.0.peers.borrow_mut()[index] = Some(AttachedPeer {
            role,
            transport: transport.downgrade(),
        });

        transport
    }

    #[cfg(feature = "security")]
    fn transport_with_security(
        &self,
        index: usize,
        role: Role,
        security: duplex_core::Security,
    ) -> Transport<Memory> {
        let transport = Transport::with_security(
            |_| Memory {
                queue: self.clone(),
                to: 1 - index,
            },
            security,
        );

        self.0.peers.borrow_mut()[index] = Some(AttachedPeer {
            role,
            transport: transport.downgrade(),
        });

        transport
    }

    fn push(&self, to: usize, batch: Rc<[u8]>) {
        let slices = if self.0.sealed.get() {
            Vec::new()
        } else {
            self.0.aliases.borrow_mut()[to].read(&batch)
        };

        self.0
            .batches
            .borrow_mut()
            .push_back(PostedBatch { to, batch, slices });
    }
}

impl Driver for Memory {
    type Scheduler = ManualScheduler;

    const CAPABILITIES: duplex_core::Capabilities = duplex_core::Capabilities::DUPLEX;

    fn transport_name(&self) -> &'static str {
        "memory"
    }

    fn send(&self, batch: OutboundBatch) {
        self.queue.push(self.to, Rc::from(batch.as_slice()));
    }

    fn scheduler(&self) -> &ManualScheduler {
        &self.queue.0.scheduler
    }

    fn random(&self) -> [u8; 16] {
        [0; 16]
    }

    fn pause(&self) {
        self.queue.set_paused(1 - self.to, true);
    }

    fn resume(&self) {
        self.queue.set_paused(1 - self.to, false);
    }
}
