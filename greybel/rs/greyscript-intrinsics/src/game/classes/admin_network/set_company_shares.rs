use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminSetCompanyShares;

/// Sets the shares bank account `bank_account` has of this network's company, as buying them on the stock market
/// does (`StocksCompany.BuyStocksCompany`). No payment is made. Returns true.
#[native(method)]
pub fn set_company_shares(
    vm: &mut Vm,
    this: &Value,
    bank_account: &Value,
    shares: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "set_company_shares";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let (Some(bank_account), Some(shares)) = (bank_account.as_str(), shares.as_int()) else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminSetCompanyShares {
        program: program(vm)?,
        public_ip: public_ip.into(),
        bank_account: bank_account.into(),
        shares: u32::try_from(shares.max(0)).unwrap_or(u32::MAX),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_set_company_shares(&message),
        |_, set: bool| set,
    ))
}
