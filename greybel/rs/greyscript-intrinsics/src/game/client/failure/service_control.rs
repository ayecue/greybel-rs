//! Grey Hack failure text for starting, stopping, and installing services.

use crate::protocol::ServiceControlFailure;

/// Formats `ServiceIntrinsics` failures.
pub(super) fn service_control_text(
    intrinsic: &str,
    failure: ServiceControlFailure,
) -> &'static str {
    match (failure, intrinsic) {
        (ServiceControlFailure::RootOnly, _) => "Denied. Only root user can install this service.",
        (ServiceControlFailure::RouterHttp, "start_service") => {
            "Error. It is not possible to manually start this service on routers."
        }
        (ServiceControlFailure::RouterHttp, "stop_service") => {
            "Error. It is not possible to manually stop this service on routers."
        }
        (ServiceControlFailure::RouterHttp, _) => {
            "Error. It is not possible to install this service on routers."
        }
    }
}
