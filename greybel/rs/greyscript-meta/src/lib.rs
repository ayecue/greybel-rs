mod data;

use std::sync::LazyLock;

use meta_utils::{Container, DEFAULT_LANGUAGE};

static CONTAINER: LazyLock<Container> = LazyLock::new(data::load);

#[must_use]
pub fn container() -> &'static Container {
    &CONTAINER
}

#[must_use]
pub fn is_native<S: AsRef<str>>(types: &[S], property: &str) -> bool {
    container()
        .definition(types, property, DEFAULT_LANGUAGE)
        .is_some()
}
