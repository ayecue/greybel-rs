slotmap::new_key_type! {
    pub(crate) struct TransportId;
    pub(crate) struct EndpointId;
    pub(crate) struct EntryId;
    pub(crate) struct NotificationId;
    pub(crate) struct HookKey;
}

/// Message or notification moved from a transport processing queue to its channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Work {
    Message(EntryId),
    Notification(NotificationId),
}

const _: () = assert!(size_of::<Option<EntryId>>() == size_of::<EntryId>());
