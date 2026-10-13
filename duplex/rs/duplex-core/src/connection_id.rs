use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Nonzero connection ID assigned by a provider.
///
/// `Option<ConnectionId>` occupies the same size as `u32`.
pub struct ConnectionId(NonZeroU32);

const _: () = assert!(size_of::<Option<ConnectionId>>() == size_of::<u32>());

impl ConnectionId {
    /// Connection ID used when hashing produces zero.
    pub const FIRST: Self = Self(NonZeroU32::MIN);

    #[must_use]
    /// Returns `None` for zero `id`, which represents no connection.
    pub const fn of(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(id) => Some(Self(id)),
            None => None,
        }
    }

    #[must_use]
    /// Returns the numeric connection ID.
    pub const fn get(self) -> u32 {
        self.0.get()
    }

    #[must_use]
    pub(crate) fn to_u32(id: Option<Self>) -> u32 {
        id.map_or(0, Self::get)
    }
}

impl Display for ConnectionId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}
