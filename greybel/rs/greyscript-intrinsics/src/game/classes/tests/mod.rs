//! `#[default]` takes only literals, so the service ports written there must equal the rules' constants.

#[path = "support/mod.rs"]
mod support;

use greybel_interpreter::NativeDefault;
use greyhack_behaviour::{DEFAULT_REPOSITORY_PORT, ServiceId};
use support::default_port;

use super::{apt_client, metaxploit};

#[test]
fn should_default_service_ports_to_the_behaviour_constants() {
    assert_eq!(
        default_port(metaxploit::METHODS, "rshell_client"),
        NativeDefault::Number(f64::from(ServiceId::Rshell.default_port()))
    );
    assert_eq!(
        default_port(apt_client::METHODS, "add_repo"),
        NativeDefault::Number(f64::from(DEFAULT_REPOSITORY_PORT))
    );
}
