use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::is_admin;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{AdminReleaseLibraryVersion, LibraryRelease};

/// Releases the next version of library file `library`, as the game does for the first versions of a world: every
/// visible weakness counts as used up, and the check for library versions patches them
/// (`VersionsControl.ForceInitialVersionGeneration`). Returns the new version, or Grey Hack's failure text. Not
/// available in Grey Hack scripts.
#[native(method)]
pub fn release_library_version(
    vm: &mut Vm,
    this: &Value,
    library: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "release_library_version";

    if !is_admin(vm, this, NAME) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(library) = library.as_str() else {
        return Err(IntrinsicError::InvalidArguments(NAME));
    };
    let message = AdminReleaseLibraryVersion {
        program: program(vm)?,
        library: library.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_release_library_version(&message),
        |_, response: LibraryRelease| match response {
            LibraryRelease::Released(version) => Ok(Value::from(version)),
            LibraryRelease::Failure(failure) => failure_value(NAME, &failure),
        },
    ))
}
