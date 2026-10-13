//! Runs futures on a single-threaded Tokio runtime with local tasks.

use std::future::Future;

use tokio::io::AsyncReadExt;
use tokio::task::{self, LocalSet};

/// Runs `future` to completion on a new current-thread runtime with all drivers and a [`LocalSet`].
pub fn run_local<F: Future>(future: F) -> F::Output {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio's runtime builds");

    LocalSet::new().block_on(&runtime, future)
}

/// Runs `future` with [`run_local`] until stdin EOF, then aborts the task.
pub fn run_until_input_ends<F>(future: F)
where
    F: Future + 'static,
    F::Output: 'static,
{
    run_local(async move {
        let handle = task::spawn_local(future);
        let mut input = Vec::new();
        let _ = tokio::io::stdin().read_to_end(&mut input).await;

        handle.abort();
    });
}
