mod data;

use std::sync::LazyLock;

use meta_utils::Container;

static CONTAINER: LazyLock<Container> = LazyLock::new(data::load);

#[must_use]
pub fn container() -> &'static Container {
    &CONTAINER
}
