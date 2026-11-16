//! Grey Hack game intrinsics backed by world-protocol operations ([`crate::protocol`]).
//! Object methods are also registered as globals under their C# names.
//!
//! Hosts register [`GAME_API`] and install the server-created program session as [`Game`] host data.
//! [`crate::GreyHackInterpreter::start`] performs both steps.

mod classes;
mod client;

pub(crate) use classes::shell::{CachedCode, LaunchCache, launch_key};
use classes::{
    apt_client, blockchain, coin, computer, crypto, ctf, debug_library, file, meta_lib, meta_mail,
    metaxploit, net_session, player, port, router, service, shell, smart_appliance, sub_wallet,
    traffic, wallet,
};
pub(crate) use client::failure::failure_text;
pub use client::game::Game;
pub(crate) use client::login_encryption::LoginEncryption;
pub(crate) use client::print::print_line;
pub use client::session::Session;
use client::state::game;
pub(super) use client::state::session_mut;
pub(crate) use client::terminal::{Terminal, TerminalOutput, send_output};
use greybel_interpreter::{NativeWithName, Register, Vm, natives};

/// Player game globals such as `get_shell` and `current_path`.
const PLAYER_GLOBALS: &[NativeWithName] = natives![
    player::get_shell::get_shell,
    player::get_router::get_router,
    player::get_switch::get_switch,
    player::include_lib::include_lib,
    player::get_custom_object::get_custom_object,
    player::get_greybel_util::get_greybel_util,
    player::nslookup::nslookup,
    player::whois::whois,
    player::active_user::active_user,
    player::home_dir::home_dir,
    player::current_path::current_path,
    player::launch_path::launch_path,
    player::program_path::program_path,
    player::get_abs_path::get_abs_path,
    player::cd::cd,
    player::parent_path::parent_path,
    player::format_columns::format_columns,
    player::current_date::current_date,
    player::command_info::command_info,
    player::is_lan_ip::is_lan_ip,
    player::is_valid_ip::is_valid_ip,
    player::user_mail_address::user_mail_address,
    player::mail_login::mail_login,
    player::user_bank_number::user_bank_number,
    player::get_ctf::get_ctf,
    player::reset_ctf_password::reset_ctf_password,
    player::user_input::user_input,
    player::poll_input,
    player::clear_screen::clear_screen,
    player::exit::exit,
];

/// Object method tables and global-name overrides.
type ClassDefinition = (
    &'static [NativeWithName],
    &'static [(&'static str, &'static str)],
);

const CLASSES: &[ClassDefinition] = &[
    (
        computer::METHODS,
        &[("local_ip", "lan_ip"), ("public_ip", "public_ip_pc")],
    ),
    (router::METHODS, &[]),
    (file::METHODS, &[]),
    (port::METHODS, &[]),
    (shell::METHODS, &[]),
    (crypto::METHODS, &[]),
    (apt_client::METHODS, &[]),
    (metaxploit::METHODS, &[]),
    (meta_lib::METHODS, &[]),
    (net_session::METHODS, &[]),
    (meta_mail::METHODS, &[("delete", "delete_mail")]),
    (blockchain::METHODS, &[]),
    (coin::METHODS, &[("reset_password", "reset_password_coin")]),
    (
        sub_wallet::METHODS,
        &[
            ("get_balance", "get_balance_subwallet"),
            ("delete", "delete_subwallet"),
        ],
    ),
    (wallet::METHODS, &[]),
    (service::METHODS, &[]),
    (ctf::METHODS, &[]),
    (smart_appliance::METHODS, &[]),
    (traffic::METHODS, &[]),
    (debug_library::METHODS, &[("scan", "scan_debuglib")]),
];

/// Returns a method's global-name override, or its type-map key when no override exists.
fn global_name(key: &'static str, overrides: &[(&'static str, &'static str)]) -> &'static str {
    overrides
        .iter()
        .find(|(from, _)| *from == key)
        .map_or(key, |(_, to)| *to)
}

/// Extends [`crate::API`] with player globals and all object methods as globals.
/// Aliased names share one native function.
pub const GAME_API: GameApi = GameApi;

/// Returns the game API as a [`Register`]; see [`GAME_API`].
#[derive(Debug, Clone, Copy)]
pub struct GameApi;

impl Register for GameApi {
    fn register(&self, vm: &mut Vm) {
        crate::API.register(vm);
        for (table, overrides) in CLASSES {
            for &(key, native) in *table {
                let value = vm.native_function(native);

                vm.define(global_name(key, overrides), value);
            }
        }

        for &(name, native) in PLAYER_GLOBALS {
            let value = vm.native_function(native);

            vm.define(name, value);
        }
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
