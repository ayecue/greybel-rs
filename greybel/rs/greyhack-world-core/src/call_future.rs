//! The future a lower crate gets back when it calls a trait a higher domain implements.
//!
//! Each call runs where core previously called the domain directly, preserving side-effect and random-draw order.
//! World futures are single-threaded and therefore need no `Send` bound.

use greyhack_world_routines::BoxFuture;

use crate::Result;

/// Future returned by a call into a higher domain.
pub type CallFuture<'a, T> = BoxFuture<'a, Result<T>>;
