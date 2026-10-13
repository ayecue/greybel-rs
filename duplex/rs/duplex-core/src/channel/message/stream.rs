use std::collections::VecDeque;
use std::task::{Context, Poll, Waker};

use super::{Body, ReceiveWindow};
use crate::error::Error;

pub(crate) enum Event {
    Start(Body),
    Item(Body),
    End(Body),
    Failure(Body),
    Error(Error),
}

impl Event {
    fn ends(&self) -> bool {
        matches!(self, Self::End(_) | Self::Failure(_) | Self::Error(_))
    }
}

pub(crate) struct InputHandle {
    next: Option<Event>,
    later: VecDeque<Event>,
    waker: Option<Waker>,
    ended: bool,
    window: ReceiveWindow,
}

impl InputHandle {
    pub(crate) fn new(window: ReceiveWindow) -> Self {
        Self {
            next: None,
            later: VecDeque::new(),
            waker: None,
            ended: false,
            window,
        }
    }

    pub(crate) fn push(&mut self, event: Event) {
        if self.ended {
            return;
        }

        if let Event::Item(body) = &event {
            self.window.on_receive(body.bytes().len());
        }

        self.ended = event.ends();
        if self.next.is_none() {
            self.next = Some(event);
        } else {
            self.later.push_back(event);
        }

        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }

    pub(crate) fn poll(&mut self, context: &mut Context<'_>) -> Poll<Event> {
        match self.next.take() {
            Some(event) => {
                self.next = self.later.pop_front();

                Poll::Ready(event)
            }
            None => {
                self.waker = Some(context.waker().clone());

                Poll::Pending
            }
        }
    }

    pub(crate) fn start(&mut self) -> Option<u32> {
        self.window.start()
    }

    pub(crate) fn consume_item(&mut self) -> Option<u32> {
        self.window.consume_item()
    }
}

#[derive(Default)]
pub(crate) struct OutputHandle {
    available: u32,
    opened: bool,
    waker: Option<Waker>,
    error: Option<Error>,
}

impl OutputHandle {
    pub(crate) fn new(count: u32) -> Self {
        let mut handle = Self::default();

        handle.grant(count);

        handle
    }

    pub(crate) fn grant(&mut self, count: u32) {
        self.available = self.available.saturating_add(count);
        self.opened |= count > 0;
        self.wake();
    }

    pub(crate) fn fail(&mut self, error: Error) {
        self.error.get_or_insert(error);
        self.wake();
    }

    pub(crate) fn poll_take(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let ready = self.poll_available(context);

        if let Poll::Ready(Ok(())) = ready {
            self.available -= 1;
        }

        ready
    }

    pub(crate) fn poll_available(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Error>> {
        if let Some(error) = &self.error {
            return Poll::Ready(Err(error.clone()));
        }

        if self.available == 0 {
            self.waker = Some(context.waker().clone());

            return Poll::Pending;
        }

        Poll::Ready(Ok(()))
    }

    pub(crate) fn poll_open(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Error>> {
        if let Some(error) = &self.error {
            return Poll::Ready(Err(error.clone()));
        }

        if !self.opened {
            self.waker = Some(context.waker().clone());

            return Poll::Pending;
        }

        Poll::Ready(Ok(()))
    }

    pub(crate) fn take_all(&mut self) -> Result<u32, Error> {
        match &self.error {
            Some(error) => Err(error.clone()),
            None => Ok(std::mem::take(&mut self.available)),
        }
    }

    pub(crate) fn release(&mut self, count: u32) {
        self.available = self.available.saturating_add(count);
    }

    pub(crate) fn is_pending(&self) -> bool {
        self.available == 0 && self.error.is_none() && self.waker.is_some()
    }

    fn wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}
