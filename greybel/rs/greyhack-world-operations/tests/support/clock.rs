use crate::support::Runtime;
use crate::support::program::{PlayerCurrentDate, ProgramId};

/// Asks `runtime`'s program `program` for the date, as `current_date` does.
pub fn current_date(runtime: &Runtime, program: ProgramId) -> impl Future<Output = String> + use<> {
    let response = runtime
        .channel
        .send(&PlayerCurrentDate { program })
        .expect("the date is requested");

    async move { response.await.expect("the game server responds") }
}
