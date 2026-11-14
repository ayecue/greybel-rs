//! `connect_service` opens a remote shell, running server-side SSH encode/decode scripts first when encryption is enabled.
//! Hand-written because those scripts execute asynchronously after the game-server response.

use greybel_interpreter::{
    ArgContainer, Native, NativeDefault, NativeFuture, NativeOutput, NativeParam, Value, Vm,
};
use greyhack_behaviour::forbidden_param_token;

use self::encryption::EncryptedLogin;
use super::shell;
use crate::error::IntrinsicError;
use crate::game::client::failure::{failure_text, invalid_password_text};
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::object::{self, Access};
use crate::game::client::server::{operate, program, server};
use crate::game::game;
use crate::protocol::{ServiceConnection, ShellConnectService};
use crate::runtime::ChildRunner;

mod encryption;

const NAME: &str = "connect_service";

static PARAMS: &[NativeParam] = &[
    NativeParam::new("ipAddress"),
    NativeParam::new("port"),
    NativeParam::new("user"),
    NativeParam::new("password"),
    NativeParam::with_default("serviceId", NativeDefault::String("ssh")),
];

pub static NATIVE: Native = Native::from_async_fn(None, PARAMS, true, call);

fn call(vm: &mut Vm, args: &ArgContainer) -> NativeFuture {
    match send(vm, args) {
        Ok(future) => future,
        Err(error) => NativeOutput::ready(Err::<Value, _>(error)),
    }
}

fn send(vm: &mut Vm, args: &ArgContainer) -> Result<NativeFuture, IntrinsicError> {
    let Some(from) = shell(vm, args.self_value(), NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    LoginEncryption::deny_in(vm, NAME)?;

    if let Some(forbidden) = args
        .get("password")
        .as_str()
        .and_then(forbidden_param_token)
    {
        return Ok(NativeOutput::ready(invalid_password_text(forbidden)));
    }

    let text = |name: &str| args.get(name).as_str().map(|text| text.to_owned().into());
    let message = ShellConnectService {
        program: program(vm)?,
        shell: from.reference(),
        address: text("ipAddress"),
        port: args.get("port").as_int(),
        user: text("user"),
        password: text("password"),
        service: text("serviceId"),
    };
    let password = args.get("password").as_str().unwrap_or_default().to_owned();
    let source = from.reference();
    let address = message.address.as_deref().unwrap_or_default().to_owned();
    let port = u16::try_from(message.port.unwrap_or_default()).unwrap_or(0);
    let service = message.service.as_deref().unwrap_or_default().to_owned();
    let request = server(vm)?.send_shell_connect_service(&message);
    let runner = vm.host_data::<ChildRunner>().cloned();
    let game = game(vm)?.clone();
    let stdout = vm.stdout().clone();

    Ok(Box::pin(async move {
        let connection = match operate(NAME, request).await {
            Ok(connection) => connection,
            Err(error) => return NativeOutput::new(Err::<Value, _>(error)),
        };

        match connection {
            ServiceConnection::Null => NativeOutput::new(Value::Null),
            ServiceConnection::Failure(failure) => {
                NativeOutput::new(failure_text("connect_service", &failure).map(Value::from))
            }
            ServiceConnection::Shell { shell, ftp } => shell_output(Access {
                object: game.server.object(shell),
                computer: None,
                ftp,
            }),
            ServiceConnection::Encrypted {
                user,
                guest_shell,
                failure,
                encode,
                decode,
            } => {
                let guest = Access::new(game.server.object(guest_shell));
                let Some(runner) = runner else {
                    return NativeOutput::new(Err::<Value, _>(IntrinsicError::NoChildRunner));
                };
                let login = EncryptedLogin {
                    runner,
                    game,
                    stdout,
                    guest,
                    source,
                    address,
                    port,
                    user,
                    service,
                    failure,
                    encode,
                    decode,
                };

                login.run(&password).await
            }
        }
    }))
}

/// Constructs a VM shell object for `access`.
fn shell_output(access: Access) -> NativeOutput {
    NativeOutput::with_vm(move |vm: &mut Vm| object::create_shell(vm, access))
}
