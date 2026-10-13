//! Per-intrinsic call-throttling stopwatches stored as VM host data.
//! Marker types distinguish Grey Hack's `Stopwatch` instances in `HelperScript` and `GreyInterpreter`.

use std::marker::PhantomData;
use std::time::Duration;

use greybel_interpreter::Vm;

/// Returns the last restart time of stopwatch `W` on the VM clock.
struct Stopwatch<W> {
    last_restart: Duration,
    marker: PhantomData<W>,
}

/// Returns integer milliseconds since stopwatch `W` restarted, matching `Stopwatch.ElapsedMilliseconds`.
/// Returns `None` if it has never run.
pub(in crate::game) fn elapsed_ms<W: 'static>(vm: &Vm) -> Option<u128> {
    let now = vm.clock().now();

    vm.host_data::<Stopwatch<W>>()
        .map(|watch| now.saturating_sub(watch.last_restart).as_millis())
}

/// Restarts stopwatch `W` at the current VM time (`Stopwatch.Restart`).
pub(in crate::game) fn restart<W: 'static>(vm: &mut Vm) {
    let now = vm.clock().now();

    restart_at::<W>(vm, now);
}

/// Restarts stopwatch `W` at VM time `at`, for calls throttled after the game-server response.
pub(in crate::game) fn restart_at<W: 'static>(vm: &mut Vm, at: Duration) {
    vm.host_data_or_insert_with(|| Stopwatch::<W> {
        last_restart: at,
        marker: PhantomData,
    })
    .last_restart = at;
}
