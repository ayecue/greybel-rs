//! Sends native operations to the game server and converts responses into native results.

use std::rc::Rc;

use duplex_core::{Error, PendingResponse, ValidatedDecode};
use greybel_interpreter::{IntoNativeResult, NativeFuture, NativeOutput, Value, Vm};
use greyhack_behaviour::Refusal;

use crate::error::IntrinsicError;
use crate::game::client::failure::{coin_status_text, failure_text, failure_value, refusal_text};
use crate::game::game;
use crate::protocol::{Failure, Outcome, ProgramChannel, ProgramId, ScriptFailure};

/// Game server receiving the program's operations.
pub(in crate::game) fn server(vm: &Vm) -> Result<Rc<dyn ProgramChannel>, IntrinsicError> {
    Ok(Rc::clone(game(vm)?.server.channel()))
}

/// VM program identified by every operation.
pub(in crate::game) fn program(vm: &Vm) -> Result<ProgramId, IntrinsicError> {
    Ok(game(vm)?.program)
}

/// Server operation outcome: result `R` or a failure raised in the program.
pub(in crate::game) type Operation<R> = PendingResponse<R, ScriptFailure>;

/// Sends `request` and runs `finish` on the VM to produce the native result.
/// Any operation failure fails the native; these operations only raise [`ScriptFailure::ConnectionInterrupted`] or [`ScriptFailure::ProgramEnded`].
pub(in crate::game) fn respond<R, T>(
    _vm: &Vm,
    request: Result<Operation<R>, Error>,
    finish: impl FnOnce(&mut Vm, R) -> T + 'static,
) -> NativeFuture
where
    R: ValidatedDecode + 'static,
    T: IntoNativeResult + 'static,
{
    settle(request, move |vm, result| match result {
        Ok(response) => Ok(finish(vm, response)),
        Err(ScriptFailure::ConnectionInterrupted) => Err(INTERRUPTION),
        Err(ScriptFailure::ProgramEnded) => Err(PROGRAM_END),
        Err(ScriptFailure::GameCrash) => Err(IntrinsicError::GameCrash),
        Err(failure) => Err(IntrinsicError::UnexpectedFailure(failure)),
    })
}

/// Variant of [`respond`] whose `finish` receives either a result or an operation failure.
/// [`ScriptFailure::Refused`] returns its text directly without calling `finish`.
pub(in crate::game) fn settle<R, T>(
    request: Result<Operation<R>, Error>,
    finish: impl FnOnce(&mut Vm, Result<R, ScriptFailure>) -> T + 'static,
) -> NativeFuture
where
    R: ValidatedDecode + 'static,
    T: IntoNativeResult + 'static,
{
    Box::pin(async move {
        let response = match request {
            Ok(request) => match request.await {
                Ok(response) => Ok(Ok(response)),
                Err(Error::Failure(failure)) => Ok(Err(failure)),
                Err(error) => Err(IntrinsicError::GameServer(error)),
            },
            Err(error) => Err(IntrinsicError::GameServer(error.with_failure())),
        };

        NativeOutput::with_vm(move |vm: &mut Vm| match response {
            Ok(Err(ScriptFailure::Refused(failure))) => Ok(Value::from(refused_text(&failure)?)),
            Ok(response) => finish(vm, response).into_native_result(vm.heap_mut()),
            Err(error) => Err(error.into()),
        })
    })
}

/// Awaits `request` for `intrinsic`: an operation failure becomes the error `intrinsic` raises ([`raise`]), any other error a game-server error.
pub(in crate::game) async fn operate<R>(
    intrinsic: &'static str,
    request: Result<Operation<R>, Error>,
) -> Result<R, IntrinsicError>
where
    R: ValidatedDecode + 'static,
{
    match request {
        Ok(request) => request.await.map_err(|error| match error {
            Error::Failure(failure) => raise(intrinsic, failure),
            error => IntrinsicError::GameServer(error),
        }),
        Err(error) => Err(IntrinsicError::GameServer(error.with_failure())),
    }
}

/// Converts a rule refusal for `intrinsic` into its return text or raised error.
pub(in crate::game) fn refuse(
    intrinsic: &'static str,
    refusal: &Refusal,
) -> Result<NativeFuture, IntrinsicError> {
    Ok(NativeOutput::ready(refusal_text(intrinsic, refusal)?))
}

/// Converts `failure` into the runtime error raised by `intrinsic`.
pub(in crate::game) fn raise(intrinsic: &'static str, failure: ScriptFailure) -> IntrinsicError {
    match failure {
        ScriptFailure::TooManyFolders => IntrinsicError::TooManyFolders(intrinsic),
        ScriptFailure::NoConnection => IntrinsicError::NoConnection(intrinsic),
        ScriptFailure::InvalidArguments => IntrinsicError::InvalidArguments(intrinsic),
        ScriptFailure::NoAccess => IntrinsicError::NoAccess(intrinsic),
        ScriptFailure::WifiCardInactive => IntrinsicError::WifiCardInactive,
        ScriptFailure::ConnectionInterrupted => INTERRUPTION,
        ScriptFailure::ProgramEnded => PROGRAM_END,
        ScriptFailure::GameCrash => IntrinsicError::GameCrash,
        ScriptFailure::Refused(failure) => IntrinsicError::UnreturnedRefusal { intrinsic, failure },
    }
}

/// Returns the error for using an object obtained before its computer rebooted.
const INTERRUPTION: IntrinsicError = IntrinsicError::Message("remote connection interrupted");

/// Returns the program-ended error, stopping a program whose server-side termination has not yet been observed locally.
const PROGRAM_END: IntrinsicError = IntrinsicError::Message("the program has ended");

/// Returns a coin method's status value when call preconditions fail (`CoinIntrinsics.CheckStatus`).
fn refused_text(failure: &Failure) -> Result<String, IntrinsicError> {
    match failure {
        Failure::Economy(failure) => Ok(coin_status_text(failure)),
        failure => failure_text("", failure),
    }
}

impl Outcome {
    /// Converts `intrinsic`'s outcome into its script value, formatting any failure for that intrinsic.
    pub(in crate::game) fn response(
        self,
        intrinsic: &'static str,
    ) -> Result<Value, IntrinsicError> {
        Ok(match self {
            Self::Null => Value::Null,
            Self::True => Value::TRUE,
            Self::False => Value::FALSE,
            Self::Failure(failure) => failure_value(intrinsic, &failure)?,
        })
    }

    /// Variant of [`Outcome::response`] for intrinsics returning `""` on success.
    pub(in crate::game) fn into_value_or_empty(
        self,
        intrinsic: &'static str,
    ) -> Result<Value, IntrinsicError> {
        match self {
            Self::True => Ok(Value::from("")),
            outcome => outcome.response(intrinsic),
        }
    }
}
