use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{
    AdminAnswerZeroDay, AdminBuyDebugOffer, AdminCreateProceduralMission, AdminCreateZeroDayChain,
    AdminDebugOffer, AdminGetMailAccount, AdminRequestZeroDayMission, AdminVisitZeroDayHackShop,
    DebugOfferInfo, MailAccountInfo, ZeroDayMission, ZeroDayOffer,
};

fn offer_value(vm: &mut Vm, offer: ZeroDayOffer) -> Value {
    match offer {
        ZeroDayOffer::Null => Value::Null,
        ZeroDayOffer::Failure(text) => Value::from(text),
        ZeroDayOffer::Offer(offer) => offer_map(vm, offer),
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "GreyScript numbers are IEEE-754 doubles, as in the game"
)]
fn offer_map(vm: &mut Vm, offer: DebugOfferInfo) -> Value {
    Value::Map(vm.heap_mut().add_map_from([
        (Value::from("seller"), Value::from(offer.seller)),
        (Value::from("user"), Value::from(offer.user)),
        (Value::from("password"), Value::from(offer.password)),
        (Value::from("price"), Value::from(offer.price as f64)),
        (
            Value::from("expires_at"),
            Value::from(offer.expires_at as f64),
        ),
        (Value::from("purchased"), Value::from(offer.purchased)),
    ]))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Prices use the game's numeric-to-integer truncation"
)]
fn integer(value: f64) -> i64 {
    value as i64
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Seeds use the game's numeric-to-integer conversion"
)]
fn unsigned(value: f64) -> u64 {
    value as u64
}

fn optional_text(
    value: Option<&Value>,
    name: &'static str,
) -> Result<Option<String>, IntrinsicError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .map(|text| Some(text.to_owned()))
            .ok_or(IntrinsicError::InvalidArguments(name)),
    }
}

#[native(method)]
pub fn debug_offer(
    vm: &mut Vm,
    this: &Value,
    user: Option<&Value>,
    password: Option<&Value>,
    price: Option<&Value>,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "debug_offer";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let user = optional_text(user, NAME)?.unwrap_or_default();
    let password = optional_text(password, NAME)?.unwrap_or_default();
    let price = price.map_or(Ok(0), |value| {
        value
            .as_number()
            .map(integer)
            .ok_or(IntrinsicError::InvalidArguments(NAME))
    })?;
    let message = AdminDebugOffer {
        program: program(vm)?,
        user: user.into(),
        password: password.into(),
        price,
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_debug_offer(&message),
        |vm, offer: ZeroDayOffer| Ok::<_, IntrinsicError>(offer_value(vm, offer)),
    ))
}

#[native(method)]
pub fn buy_debug_offer(
    vm: &mut Vm,
    this: &Value,
    seller: &Value,
    price: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "buy_debug_offer";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(seller), Some(price)) = (seller.as_str(), price.as_number()) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminBuyDebugOffer {
        program: program(vm)?,
        seller: seller.into(),
        price: integer(price),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_buy_debug_offer(&message),
        |vm, offer: ZeroDayOffer| Ok::<_, IntrinsicError>(offer_value(vm, offer)),
    ))
}

#[native(method)]
pub fn create_zero_day_chain(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    if !is_admin(vm, this, "create_zero_day_chain") {
        return Ok(NativeOutput::ready(Value::Null));
    }

    Ok(respond(
        vm,
        server(vm)?.send_admin_create_zero_day_chain(&AdminCreateZeroDayChain {
            program: program(vm)?,
        }),
        |_, address: Option<String>| {
            Ok::<_, IntrinsicError>(address.map_or(Value::Null, Value::from))
        },
    ))
}

#[native(method)]
pub fn create_procedural_mission(
    vm: &mut Vm,
    this: &Value,
    seed: Option<&Value>,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "create_procedural_mission";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let seed = seed.map_or(Ok(0), |value| {
        value
            .as_number()
            .map(unsigned)
            .ok_or(IntrinsicError::InvalidArguments(NAME))
    })?;

    Ok(respond(
        vm,
        server(vm)?.send_admin_create_procedural_mission(&AdminCreateProceduralMission {
            program: program(vm)?,
            seed,
        }),
        |_, address: Option<String>| {
            Ok::<_, IntrinsicError>(address.map_or(Value::Null, Value::from))
        },
    ))
}

#[native(method)]
pub fn request_zero_day_mission(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "request_zero_day_mission";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    Ok(respond(
        vm,
        server(vm)?.send_admin_request_zero_day_mission(&AdminRequestZeroDayMission {
            program: program(vm)?,
        }),
        |vm, result: ZeroDayMission| {
            Ok::<_, IntrinsicError>(match result {
                ZeroDayMission::Failure(text) => Value::from(text),
                ZeroDayMission::Rumor(rumor) => Value::Map(vm.heap_mut().add_map_from([
                    (Value::from("public_ip"), Value::from(rumor.public_ip)),
                    (Value::from("lan_ip"), Value::from(rumor.lan_ip)),
                    (Value::from("sender"), Value::from(rumor.sender)),
                    (Value::from("body"), Value::from(rumor.body)),
                ])),
            })
        },
    ))
}

#[native(method)]
pub fn answer_zero_day(
    vm: &mut Vm,
    this: &Value,
    public_ip: &Value,
    user: &Value,
    from_mail: &Value,
    answer: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "answer_zero_day";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let (Some(public_ip), Some(user), Some(from_mail), Some(answer)) = (
        public_ip.as_str(),
        user.as_str(),
        from_mail.as_str(),
        answer.as_str(),
    ) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    if answer != "GOOD" && answer != "BAD" {
        return Err(IntrinsicError::InvalidArguments(NAME));
    }
    let message = AdminAnswerZeroDay {
        program: program(vm)?,
        public_ip: public_ip.into(),
        user: user.into(),
        from_mail: from_mail.into(),
        answer: answer.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_answer_zero_day(&message),
        |_, answer: Option<String>| {
            Ok::<_, IntrinsicError>(answer.map_or(Value::Null, Value::from))
        },
    ))
}

#[native(method)]
pub fn visit_zero_day_hack_shop(
    vm: &mut Vm,
    this: &Value,
    public_ip: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "visit_zero_day_hack_shop";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(public_ip) = public_ip.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminVisitZeroDayHackShop {
        program: program(vm)?,
        public_ip: public_ip.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_visit_zero_day_hack_shop(&message),
        |_, available: bool| Ok::<_, IntrinsicError>(Value::from(available)),
    ))
}

#[native(method)]
pub fn get_mail_account(
    vm: &mut Vm,
    this: &Value,
    address: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "get_mail_account";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(address) = address.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_get_mail_account(&AdminGetMailAccount {
            program: program(vm)?,
            address: address.into(),
        }),
        |vm, account: Option<MailAccountInfo>| {
            Ok::<_, IntrinsicError>(account.map_or(Value::Null, |account| {
                Value::Map(vm.heap_mut().add_map_from([
                    (Value::from("address"), Value::from(account.address)),
                    (
                        Value::from("password_hash"),
                        Value::from(account.password_hash),
                    ),
                ]))
            }))
        },
    ))
}
