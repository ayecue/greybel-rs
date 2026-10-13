//! `netSession` objects from `net_use`, covering computer service libraries and routers, one file per method.

use greybel_interpreter::{Native, Value, Vm};

use crate::game::client::receiver;
use crate::game::client::receiver::receiver;
use crate::runtime::Handle;

mod dump_lib;
mod flood_connection;
mod get_num_conn_gateway;
mod get_num_portforward;
mod get_num_users;
mod is_any_active_user;
mod is_root_active_user;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("dump_lib", &dump_lib::dump_lib::NATIVE),
    ("get_num_users", &get_num_users::get_num_users::NATIVE),
    (
        "get_num_portforward",
        &get_num_portforward::get_num_portforward::NATIVE,
    ),
    (
        "get_num_conn_gateway",
        &get_num_conn_gateway::get_num_conn_gateway::NATIVE,
    ),
    (
        "is_any_active_user",
        &is_any_active_user::is_any_active_user::NATIVE,
    ),
    (
        "is_root_active_user",
        &is_root_active_user::is_root_active_user::NATIVE,
    ),
    (
        "flood_connection",
        &flood_connection::flood_connection::NATIVE,
    ),
];

/// Returns session object `this`.
fn session<'v>(vm: &'v Vm, this: &Value) -> Option<&'v Handle> {
    receiver::<receiver::NetSession>(vm, this)
}
