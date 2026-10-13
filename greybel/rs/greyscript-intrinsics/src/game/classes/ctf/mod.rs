//! CTF object wrappers returned by `get_ctf`.

use greybel_interpreter::{Native, Value, Vm};

use crate::error::IntrinsicError;
use crate::game::client::object;
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::receiver::{self, receiver_for};
use crate::protocol::ObjectId;
use crate::runtime::Handle;

mod get_creator_name;
mod get_description;
mod get_mail_content;
mod get_template;
mod player_success;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("get_description", &get_description::get_description::NATIVE),
    ("get_template", &get_template::get_template::NATIVE),
    ("player_success", &player_success::player_success::NATIVE),
    (
        "get_creator_name",
        &get_creator_name::get_creator_name::NATIVE,
    ),
    (
        "get_mail_content",
        &get_mail_content::get_mail_content::NATIVE,
    ),
];

/// Constructs the CTF object for server-issued `id`.
pub(crate) fn create(vm: &mut Vm, id: ObjectId) -> Result<Value, IntrinsicError> {
    object::create_of(vm, Kind::Ctf, GameObject::Ctf, id)
}

/// Returns the server identity of the CTF object.
fn ctf(vm: &Vm, this: &Value, method: &str) -> Option<ObjectId> {
    receiver_for::<receiver::Ctf>(vm, this, method).map(Handle::reference)
}
