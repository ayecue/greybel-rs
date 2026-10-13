use duplex_core::{Responder, ValidatedEncode};

use crate::protocol::{Build, BuildFailure, BuildResult, Import};

/// A build or import request that uses editor files, processed in submission order.
pub(super) enum Task {
    Build(Build<'static>, Responder<BuildResult, BuildFailure>),
    Import(Import<'static>, Responder<u32, BuildFailure>),
}

/// Maps a `Result` to either a successful response or its declared failure.
pub(super) trait Settle<R, F> {
    fn settle(self, result: Result<R, F>);
}

impl<R: ValidatedEncode, F: ValidatedEncode> Settle<R, F> for Responder<R, F> {
    fn settle(self, result: Result<R, F>) {
        match result {
            Ok(response) => self.respond(response),
            Err(failure) => self.fail(failure),
        }
    }
}
