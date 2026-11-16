//! `build` compiles source into an executable binary.
//! The game server validates files and inlines imports, the program compiles the result, and the server writes the binary.

use duplex_core::Error;
use greybel_core::ErrorKind;
use greybel_documents::DocumentError;
use greybel_interpreter::{
    ArgContainer, InterpreterError, Native, NativeDefault, NativeFuture, NativeOutput, NativeParam,
    Value, Vm,
};

use crate::error::IntrinsicError;
use crate::game::client::failure::failure_text;
use crate::game::client::server::{raise, server};
use crate::game::game;
use crate::protocol::{BuildSource, ShellBuild, ShellWriteBinary};
use crate::runtime::ChildRunner;

static PARAMS: &[NativeParam] = &[
    NativeParam::new("pathSource"),
    NativeParam::new("pathBinary"),
    NativeParam::with_default("allowImport", NativeDefault::Number(0.0)),
];

pub static NATIVE: Native = Native::from_async_fn(None, PARAMS, true, call);

fn call(vm: &mut Vm, args: &ArgContainer) -> NativeFuture {
    let Some(access) = super::shell(vm, args.self_value(), "build") else {
        return NativeOutput::ready("build: Invalid arguments");
    };
    let (Some(source_path), Some(folder), Some(allow)) = (
        args.get("pathSource").as_str(),
        args.get("pathBinary").as_str(),
        args.get("allowImport").as_number(),
    ) else {
        return NativeOutput::ready(Value::Null);
    };

    if source_path.is_empty() || folder.is_empty() {
        return NativeOutput::ready("pathSource and pathBinary can't be empty");
    }

    let Some(runner) = vm.host_data::<ChildRunner>().cloned() else {
        return NativeOutput::ready("Invalid shell");
    };
    let Ok(game) = game(vm) else {
        return NativeOutput::ready(Value::Null);
    };
    let Ok(server) = server(vm) else {
        return NativeOutput::ready(Value::Null);
    };
    let (source_path, folder) = (source_path.to_owned(), folder.to_owned());
    let program = game.program;
    let build = ShellBuild {
        program,
        shell: access.reference(),
        source: source_path.clone().into(),
        folder: folder.clone().into(),
    };
    let request = server.send_shell_build(&build);

    Box::pin(async move {
        let unreachable =
            |error| NativeOutput::new(Err::<Value, _>(IntrinsicError::GameServer(error)));
        let source = match request {
            Ok(response) => response.await,
            Err(error) => return unreachable(error.with_failure()),
        };
        let (source, name, source_map) = match source {
            Ok(BuildSource::Code {
                source,
                name,
                source_map,
            }) => (source, name, source_map),
            Ok(BuildSource::Failure(failure)) => {
                return NativeOutput::new(failure_text("build", &failure).map(Value::from));
            }
            Err(Error::Failure(failure)) => {
                return NativeOutput::new(Err::<Value, _>(raise("build", failure)));
            }
            Err(error) => return unreachable(error),
        };

        if let Err(error) = runner.compile_check(&source_path, source.clone()).await {
            return NativeOutput::new(Value::from(compile_failure(&error)));
        }

        let write = ShellWriteBinary {
            program,
            shell: access.reference(),
            folder: folder.into(),
            name: name.into(),
            source: source.into(),
            allow_import: allow != 0.0,
            source_map,
        };
        let result = match server.send_shell_write_binary(&write) {
            Ok(response) => response.await,
            Err(error) => return unreachable(error.with_failure()),
        };

        match result {
            Ok(outcome) => NativeOutput::new(outcome.into_value_or_empty("build")),
            Err(error) => unreachable(error),
        }
    })
}

/// Formats `error` as Grey Hack's `build` result: `<color=red>Compiler Error: <message> [line <n>]</color>` plus a line break.
/// MiniScript lexer failures use `Lexer Error`; line numbers come from MiniScript's parser.
fn compile_failure(error: &InterpreterError) -> String {
    let description = match error {
        InterpreterError::Documents(DocumentError::Parse { error, .. }) => {
            let kind = match error.diagnostic.reported_kind() {
                ErrorKind::Lexer => "Lexer",
                ErrorKind::Parser => "Compiler",
            };

            match error.reported_line {
                Some(line) => format!("{kind} Error: {error} [line {line}]"),
                None => format!("{kind} Error: {error}"),
            }
        }
        error => format!("Compiler Error: {error}"),
    };

    format!("<color=red>{description}</color>\n")
}
