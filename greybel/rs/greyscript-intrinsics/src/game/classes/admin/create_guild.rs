use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminCreateGuild;

/// Creates guild `name` that has company shares under bank account `bank_account`, with `total_exp` experience and
/// `investment_perks` investment licences and `admin_restore_perks` Admin Restore perks (none when omitted), and the
/// player as a member when `join` is not 0. Guilds are made in the
/// game's interface only. Returns true or the failure text. Not available in Grey Hack scripts.
#[native(method)]
#[allow(clippy::too_many_arguments)]
pub fn create_guild(
    vm: &mut Vm,
    this: &Value,
    name: &Value,
    bank_account: &Value,
    join: &Value,
    total_exp: &Value,
    investment_perks: &Value,
    #[default(0)] admin_restore_perks: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "create_guild";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(name), Some(bank_account), Some(join), Some(total_exp), Some(investment_perks)) = (
        name.as_str(),
        bank_account.as_str(),
        join.as_number(),
        total_exp.as_int(),
        investment_perks.as_int(),
    ) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let Some(admin_restore_perks) = admin_restore_perks.as_int() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminCreateGuild {
        program: program(vm)?,
        name: name.into(),
        bank_account: bank_account.into(),
        join: join != 0.0,
        total_exp: u32::try_from(total_exp.max(0)).unwrap_or(u32::MAX),
        investment_perks: u32::try_from(investment_perks.max(0)).unwrap_or(u32::MAX),
        admin_restore_perks: u32::try_from(admin_restore_perks.max(0)).unwrap_or(u32::MAX),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_create_guild(&message),
        |_, outcome| outcome.response(NAME),
    ))
}
