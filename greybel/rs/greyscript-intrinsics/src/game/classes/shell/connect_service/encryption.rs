//! Server-side SSH encode/decode phases of an encrypted `connect_service` login.

use std::rc::Rc;

use greybel_interpreter::{InterpreterError, NativeOutput, PrintOptions, Stdout, Value};
use greyhack_behaviour::{self as sshd, GUEST_USER, SSH_SCRIPT_FOLDER, SSH_SCRIPT_PARAM, path};

use super::{NAME, shell_output};
use crate::error::IntrinsicError;
use crate::game::client::object::Access;
use crate::game::client::server::operate;
use crate::game::{Game, Session};
use crate::protocol::{
    LoginFailure, ObjectId, ServiceLoginResult, ShellCompleteEncryptedLogin, SourceFile,
};
use crate::runtime::ChildRunner;

/// Login encrypted according to the server's SSH configuration.
pub(super) struct EncryptedLogin {
    pub(super) runner: ChildRunner,
    pub(super) game: Game,
    pub(super) stdout: Rc<dyn Stdout>,
    /// Server guest shell used to execute the scripts.
    pub(super) guest: Access,
    pub(super) source: ObjectId,
    pub(super) address: String,
    pub(super) port: u16,
    /// User attempting the login.
    pub(super) user: String,
    pub(super) service: String,
    pub(super) failure: Option<LoginFailure>,
    pub(super) encode: SourceFile,
    pub(super) decode: SourceFile,
}

impl EncryptedLogin {
    /// Runs encode on the original password and decode on its output, then completes login with both results (`HelperScript.SendPendingSshSec`, `ShellIntrinsics.connect_service`).
    pub(super) async fn run(self, password: &str) -> NativeOutput {
        let encoded_password = match self
            .phase(EncryptionMethod::Encode, &self.encode, password)
            .await
        {
            Ok(encoded_password) => encoded_password,
            Err(error) => return self.failure(error),
        };
        let decoded_password = match self
            .phase(EncryptionMethod::Decode, &self.decode, &encoded_password)
            .await
        {
            Ok(decoded_password) => decoded_password,
            Err(error) => return self.failure(error),
        };

        match self
            .complete(password, &encoded_password, &decoded_password)
            .await
        {
            Ok(ServiceLoginResult::Shell { shell, ftp }) => shell_output(Access {
                object: self.game.server.object(shell),
                computer: None,
                ftp,
            }),
            Ok(ServiceLoginResult::Null) => NativeOutput::new(Value::Null),
            Err(error) => NativeOutput::new(Err::<Value, _>(error)),
        }
    }

    /// Sends both script results to the server.
    /// The server revalidates a changed decoded password and records the encoded password like `PlayerUtils.ConnectService`.
    async fn complete(
        &self,
        password: &str,
        encoded_password: &str,
        decoded_password: &str,
    ) -> Result<ServiceLoginResult, IntrinsicError> {
        let message = ShellCompleteEncryptedLogin {
            program: self.game.program,
            shell: self.source,
            target: self.guest.reference(),
            address: self.address.clone().into(),
            port: self.port,
            user: self.user.clone().into(),
            password: password.to_owned().into(),
            service: self.service.clone().into(),
            failure: self.failure.clone(),
            encoded_password: encoded_password.to_owned().into(),
            decoded_password: decoded_password.to_owned().into(),
        };
        let request = self
            .game
            .server
            .channel()
            .send_shell_complete_encrypted_login(&message);

        operate(NAME, request).await
    }

    /// On phase failure, prints background output and returns no shell without recording a sniffer packet (`HelperScript.HelperSshSec.GetBgSalida`, `PlayerUtils.ConnectService`).
    fn failure(&self, error: EncryptionPhaseError) -> NativeOutput {
        let output = match error {
            EncryptionPhaseError::Execution(error) => error.to_string(),
            EncryptionPhaseError::MissingResult(output) => output,
        };

        self.stdout.print(
            &format!("Encryption failed: {output}"),
            PrintOptions::default(),
        );

        NativeOutput::new(Value::Null)
    }

    /// Runs a server-side SSH encode/decode script and extracts its result or error output.
    /// Both scripts run in the server guest shell from `/server/conf` with program path `/`, launched by the connecting program with parameters `ssh_enc <input>` (`HelperScript.SendPendingSshSec`).
    async fn phase(
        &self,
        method: EncryptionMethod,
        script: &SourceFile,
        input: &str,
    ) -> Result<String, EncryptionPhaseError> {
        let mut game = self.game.child(
            Session {
                shell: self.guest.clone(),
                user: GUEST_USER.to_owned(),
                current_path: SSH_SCRIPT_FOLDER.to_owned(),
                launch_path: self.game.session.program_path.clone(),
                program_path: path::ROOT.to_owned(),
            },
            self.game.program,
        );

        game.encrypts_login = true;

        // `PlayerUtils.ConfigCodeEnc` appends the call producing `HelperScript.HelperSshSec.Completed`'s marker.
        // `HelperScript.SendPendingSshSec` inserts parameters before that call.
        let source = format!(
            "params = [\"{SSH_SCRIPT_PARAM}\",\"{input}\"];{}\nargs = \"{input}\"\nprint(\">END;\" + {}(args) + \";1\")",
            script.source,
            method.name(),
        );
        let params = vec![SSH_SCRIPT_PARAM.to_owned(), input.to_owned()];
        let capture = self
            .runner
            .run_and_capture(path::ROOT, source, params, game)
            .await;

        if let Some(error) = &capture.error {
            self.report_error(error);
        }

        if let Some(result) = sshd::end_result(&capture.output) {
            return Ok(result.to_owned());
        }

        match capture.error {
            Some(InterpreterError::Runtime(_)) | None => Err(EncryptionPhaseError::MissingResult(
                sshd::before_end(&capture.output).to_owned(),
            )),
            Some(error) => Err(EncryptionPhaseError::Execution(error)),
        }
    }

    /// Sends background-VM runtime errors to the invoking terminal instead of captured output (`HelperScript.AddOutput`).
    fn report_error(&self, error: &InterpreterError) {
        let InterpreterError::Runtime(error) = error else {
            return;
        };
        let location = error
            .stack
            .first()
            .map(|at| format!(" [{} line {}]", at.path, at.line))
            .unwrap_or_default();

        self.stdout.print(
            &format!("<color=red>Runtime Error: {error}{location}</color>"),
            PrintOptions::default(),
        );
    }
}

/// Function called by `PlayerUtils.ConfigCodeEnc` after the server method body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EncryptionMethod {
    Encode,
    Decode,
}

impl EncryptionMethod {
    const fn name(self) -> &'static str {
        match self {
            Self::Encode => "Encode",
            Self::Decode => "Decode",
        }
    }
}

/// Reason an SSH encode/decode phase produced no password.
#[derive(Debug)]
enum EncryptionPhaseError {
    /// Script execution failed, for example during compilation.
    Execution(InterpreterError),
    /// Script completed without printing its result; contains prior output.
    MissingResult(String),
}
