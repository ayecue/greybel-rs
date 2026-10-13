use greybel_interpreter::{NativeFuture, Value, Vm, native};
use greyhack_behaviour::path;

use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::server::{program, respond, server};
use crate::game::{game, session_mut};
use crate::protocol::{FolderLookup, PlayerCd};

/// Changes the current folder.
/// Matches Grey Hack by resolving even relative paths from the root.
#[native]
pub fn cd(vm: &mut Vm, #[default("")] path: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(path) = path.as_str() else {
        return Err(IntrinsicError::InvalidArguments("cd"));
    };

    LoginEncryption::deny_in(vm, "cd")?;

    let session = &game(vm)?.session;
    let message = PlayerCd {
        program: program(vm)?,
        current: session.current_path.clone().into(),
        path: path.to_owned().into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_player_cd(&message),
        |vm, change| {
            let Some(change) = change else {
                return Ok(NO_SUCH_FOLDER.to_owned());
            };
            let launcher = game(vm)?.launcher_folder.clone();
            let session = session_mut(vm)?;

            if !change.current_exists {
                path::ROOT.clone_into(&mut session.current_path);
            }

            Ok::<_, IntrinsicError>(match change.target {
                FolderLookup::Found(folder) => {
                    if let Some(launcher) = launcher {
                        *launcher.borrow_mut() = Some(folder.clone());
                    }

                    session.current_path = folder;

                    String::new()
                }
                FolderLookup::Missing => NO_SUCH_FOLDER.to_owned(),
                FolderLookup::Denial => PERMISSION_ERROR.to_owned(),
            })
        },
    ))
}

const NO_SUCH_FOLDER: &str = "Error: No such file or directory";

const PERMISSION_ERROR: &str = "Error: permission denied";
