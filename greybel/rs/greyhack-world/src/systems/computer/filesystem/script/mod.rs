//! What scripts do to files through `FileIntrinsics`: reading, writing, creating, copying, moving, and
//! deleting them.

pub(crate) mod content;
pub(crate) mod create;
pub(crate) mod failure;
pub(crate) mod transfer;
