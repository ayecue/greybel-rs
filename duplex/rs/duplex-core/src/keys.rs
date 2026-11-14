slotmap::new_key_type! {
    pub(crate) struct TransportId;
    pub(crate) struct EndpointId;
    pub(crate) struct EntryId;
    pub(crate) struct NotificationId;
    pub(crate) struct HookKey;
}

/// Message, notification, or received response moved from a transport processing queue to its channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Work {
    Message(EntryId),
    Notification(NotificationId),
    Settle(EntryId),
}

const _: () = assert!(size_of::<Option<EntryId>>() == size_of::<EntryId>());
