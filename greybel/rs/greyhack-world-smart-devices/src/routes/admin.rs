//! Traffic-accident operation of the global `admin.so` object.

use greyhack_world_router::RouteTable;
use greyhack_world_router::protocol::AdminTrafficAccident;

use crate::SmartDevicesContext;

pub(crate) fn register<W: SmartDevicesContext + 'static>(routes: &RouteTable) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    _: AdminTrafficAccident| {
            Ok(ctx
                .modules
                .smart_devices()
                .traffic
                .traffic_accident()
                .await?)
        },
    );
}
