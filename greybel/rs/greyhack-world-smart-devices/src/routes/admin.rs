//! Traffic-accident operation of the global `admin.so` object.

use greyhack_world_programs::protocol::AdminTrafficAccident;
use greyhack_world_router::RouteTable;

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    _: AdminTrafficAccident| {
            Ok(ctx
                .modules
                .module::<crate::SmartDevicesModule>()
                .traffic
                .traffic_accident()
                .await?)
        },
    );
}
