# greyhack-setup-wizard library

Shared setup operations used by the graphical/CLI wizard and the headless test
runner: locate Grey Hack, install BepInEx and the matching message hook, and
write its port configuration.

Use the [frontends](../../../apps/dotnet/greyhack-setup-wizard/README.md) or
follow the [integration guide](../../../docs/greyhack-integration.md).

## Prerequisites and discovery

The library and its tests use the .NET 10 SDK. Game selection is `--game` in a
frontend, then `GREY_HACK_PATH`, then Steam libraries containing
`appmanifest_605230.acf`.

## Installation constraints

| BepInEx major | Pinned download |
|---|---|
| 5 (stable) | 5.4.23.6, `win_x64`, `macos_universal`, `linux_x64` |
| 6 (prerelease) | 6.0.0-be.791 (`+b782da4`), Unity.Mono `win-x64`, `macos-x64`, `linux-x64` |

`BepInExRelease` records each platform archive's URL and SHA-256. Downloads are
cached and checked before installation. Installing BepInEx replaces its loader
and `core`, while preserving user `plugins` and `config` folders. BepInEx 6 on
macOS requires x64 execution; the installer sets `archpreference="x86_64"` in
`run_bepinex.sh` so Apple Silicon uses Rosetta.

The hook assemblies are `GreyHackMessageHook5.dll` for BepInEx 5 and
`GreyHackMessageHook.dll` for 6. Installing one removes the other major's hook
build and installs only the hook's own files; a duplicate Newtonsoft.Json would
conflict with the game's copy. A release install downloads from
`ayecue/greybel-rs` by default (`--repository` overrides it) and verifies
GitHub's asset digest.

Frontends must obtain consent before downloads. `--yes` gives it in the CLI;
without a terminal, no download is assumed to be approved. A manually obtained
BepInEx archive can be installed with `--zip`, and a local hook with
`--assembly`.

The default message-hook port is 7777, stored as `General.Port` in
`BepInEx/config/GreyHackMessageHook.cfg`.

## Test

From this folder:

```sh
dotnet test --solution GreyHackSetupWizard.slnx
```

Or run `make test` in `greybel/dotnet`. Tests use fake games, archives and
GitHub responses rather than a real installation or network.
