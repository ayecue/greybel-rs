use greybel_interpreter::{Native, NativeDefault};

/// Returns the default of the `port` parameter of method `name`.
pub fn default_port(methods: &[(&str, &Native)], name: &str) -> NativeDefault {
    let native = methods
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, native)| native)
        .expect("method");

    native
        .params()
        .iter()
        .find(|param| param.name() == "port")
        .expect("port parameter")
        .default()
}
