//! Sets `MESSAGE_HOOK_VERSION` to the message hook release version whose game message protocol matches this build.

use std::path::Path;

const PROJECT: &str = "../../dotnet/greyhack-message-hook/src/GreyHackMessageHook.csproj";

fn main() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join(PROJECT);
    let text = std::fs::read_to_string(&project)
        .unwrap_or_else(|error| panic!("{}: {error}", project.display()));
    let version = text
        .split_once("<Version>")
        .and_then(|(_, remainder)| remainder.split_once("</Version>"))
        .map_or_else(
            || panic!("{} has no <Version>", project.display()),
            |(version, _)| version.trim(),
        );

    println!("cargo::rerun-if-changed={}", project.display());
    println!("cargo::rustc-env=MESSAGE_HOOK_VERSION={version}");
}
