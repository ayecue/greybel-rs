use greybel_interpreter::{Value, native};

#[native]
pub fn is_valid_ip(#[name = "ipAddress"] address: &Value) -> bool {
    address
        .as_str()
        .is_some_and(greyhack_behaviour::is_valid_ip)
}
