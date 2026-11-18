mod data;
mod editor_api;

use std::sync::LazyLock;

use meta_utils::Container;

static CONTAINER: LazyLock<Container> = LazyLock::new(data::load);

#[must_use]
pub fn container() -> &'static Container {
    &CONTAINER
}

#[must_use]
pub fn is_api_function(name: &str) -> bool {
    editor_api::EDITOR_API_NAMES.binary_search(&name).is_ok()
}
