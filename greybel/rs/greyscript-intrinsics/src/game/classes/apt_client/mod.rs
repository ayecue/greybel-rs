//! `aptclient` package-source and installation intrinsics (`AptClientIntrinsics`), one file per method.

use greybel_interpreter::{Native, Value, Vm};

use crate::game::client::receiver;
use crate::game::client::receiver::receiver;
use crate::runtime::Handle;

mod add_repo;
mod check_upgrade;
mod del_repo;
mod install;
mod search;
mod show;
mod update;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("add_repo", &add_repo::add_repo::NATIVE),
    ("del_repo", &del_repo::del_repo::NATIVE),
    ("install", &install::install::NATIVE),
    ("search", &search::search::NATIVE),
    ("show", &show::show::NATIVE),
    ("update", &update::update::NATIVE),
    ("check_upgrade", &check_upgrade::check_upgrade::NATIVE),
];

pub(crate) fn access(vm: &Vm, this: &Value) -> Option<Handle> {
    receiver::<receiver::AptClient>(vm, this).cloned()
}
