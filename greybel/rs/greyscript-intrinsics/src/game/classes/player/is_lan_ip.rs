use greybel_interpreter::{Value, native};
use greyhack_behaviour::is_valid_lan_ip;

#[native]
pub fn is_lan_ip(address: &Value) -> bool {
    address.as_str().is_some_and(is_valid_lan_ip)
}
