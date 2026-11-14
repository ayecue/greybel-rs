//! Persistence of sniffer and flood subscribers.

pub(crate) mod schema;
mod subscribers;

pub(crate) use subscribers::SubscriberRepository;
