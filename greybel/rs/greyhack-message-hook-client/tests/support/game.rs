//! Runs scripts in headless Grey Hack using [`greybel_test_support::game`].

use greybel_test_support::game::RUN_TIME;
use greyhack_message_hook_client::protocol::greyhack::{
    ContextBreakpointRpc, DisposedContextRpc, FinishedContextRpc,
};
use greyhack_message_hook_client::{
    Client, Context, ContextBreakpoint, ContextOptions, GreybelNotification, Payload, Subscription,
};

/// Host-side project root for script files, independent of paths on the game's computer.
pub const PROJECT: &str = "/project";
pub const FILEPATH: &str = "/project/test.src";

pub fn script(source: &str) -> ContextOptions {
    let mut options = ContextOptions::new(source, FILEPATH, PROJECT);

    options.debug_mode = false;

    options
}

/// Returns script options with debug mode enabled and a breakpoint at `line`.
pub fn stopping(source: &str, line: i32) -> ContextOptions {
    let mut options = script(source);

    options.debug_mode = true;
    options.breakpoints = vec![ContextBreakpoint {
        filepath: FILEPATH.to_owned(),
        line_num: line,
    }];

    options
}

/// Creates a context running `source` with a breakpoint at `line` and returns its initial event subscription.
pub async fn stopping_context(client: &Client, source: &str, line: i32) -> (Context, Subscription) {
    let context = client
        .create_context(stopping(source, line))
        .await
        .expect("the context is created");
    let events = context.events();

    (context, events)
}

/// Waits for the next breakpoint notification, panicking if the context finishes first.
pub async fn stop(events: &mut Subscription) -> ContextBreakpointRpc<'static> {
    loop {
        match events.next().await.expect("the context runs") {
            GreybelNotification::ContextBreakpointRpc(breakpoint) => return breakpoint,
            GreybelNotification::FinishedContextRpc(_) => {
                panic!("the context finished without stopping")
            }
            _ => {}
        }
    }
}

/// Waits for `DisposedContextRpc`, with a `RUN_TIME` timeout.
pub async fn disposal(events: &mut Subscription) -> DisposedContextRpc<'static> {
    let wait = async {
        loop {
            if let GreybelNotification::DisposedContextRpc(disposed) =
                events.next().await.expect("the context runs")
            {
                return disposed;
            }
        }
    };

    tokio::time::timeout(RUN_TIME, wait)
        .await
        .expect("the context is disposed in time")
}

/// Captured print frames and final context notification.
pub struct Run {
    pub prints: Vec<String>,
    pub finished: FinishedContextRpc<'static>,
}

impl Run {
    /// Joins print frames with newlines; each frame contains output emitted by the game in one batch.
    pub fn output(&self) -> String {
        self.prints.join("\n")
    }
}

/// Collects prints until the context finishes, with a `RUN_TIME` timeout.
pub async fn run_to_end(context: &Context) -> Run {
    collect_to_end(context.events()).await
}

/// Collects the prints of `events` until the context finishes, with a `RUN_TIME` timeout.
async fn collect_to_end(mut events: Subscription) -> Run {
    let mut prints = Vec::new();
    let collect = async {
        while let Some(response) = events.next().await {
            match response {
                GreybelNotification::PrintSentClientRpc(print) => {
                    prints.push(print.zip_output.text())
                }
                GreybelNotification::FinishedContextRpc(finished) => return finished,
                _ => {}
            }
        }

        panic!("the client closed before the context finished");
    };
    let finished = tokio::time::timeout(RUN_TIME, collect)
        .await
        .expect("the context finishes in time");

    Run { prints, finished }
}

/// Disables debug mode on a stopped context, then runs it to completion.
pub async fn resume_to_end(context: &Context) -> Run {
    // Subscribed first: the program can print and finish before turning the debug mode off returns.
    let events = context.events();

    context
        .set_debug_mode(false)
        .await
        .expect("the debug mode is turned off");

    collect_to_end(events).await
}
