//! Network sessions and failures produced by Metaxploit.

pub mod failure;
mod session;

pub use failure::{NetUseFailure, RshellFailure};
pub use session::NetUseSession;
