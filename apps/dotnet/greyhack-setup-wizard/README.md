# greyhack-setup-wizard frontends

Graphical and command-line frontends for installing BepInEx, the Grey Hack
message hook and its port configuration. Both use the shared
[`greyhack-setup-wizard` library](../../../greybel/dotnet/greyhack-setup-wizard/README.md).

See the [Grey Hack integration guide](../../../docs/greyhack-integration.md)
for a complete setup and connection walkthrough.

## Prerequisites

- .NET 10 SDK.
- An installed Grey Hack game. Use `--game`, `GREY_HACK_PATH`, or Steam library
  discovery (app 605230).
- Network access for downloads, or a local BepInEx zip/plugin DLL.

## Run

From this folder:

```sh
dotnet run --project gui
dotnet publish gui -c Release -o out

dotnet run --project cli -- locate [--game D]
dotnet run --project cli -- bepinex install --bepinex 5|6 [--zip Z] [--yes] [--cache D] [--game D]
dotnet run --project cli -- bepinex remove [--game D]
dotnet run --project cli -- plugin install (--assembly F | --release TAG|latest) [--bepinex 5|6] [--repository OWNER/REPO] [--yes] [--cache D] [--game D]
dotnet run --project cli -- plugin remove [--game D]
dotnet run --project cli -- configure --port N [--game D]
```

From the repository root, `make setup-wizard` opens the GUI.

The GUI checks the current installation before offering BepInEx, plugin and
port changes. Nothing downloads or changes until **Apply**. Downloads otherwise
require an interactive confirmation or `--yes`; a non-interactive CLI does not
download implicitly. BepInEx archives and release plugin assets are hash-checked.

Both frontends accept the recursive `--language <name>` option, such as `en` or
`de-DE`. Untranslated languages fall back to the English `.resx` resources.

## Tests

```sh
dotnet test --solution GreyHackSetupWizard.App.slnx
WIZARD_SCREENSHOTS=<folder> dotnet test --solution GreyHackSetupWizard.App.slnx
```

The tests use fake game folders, pickers and GitHub responses. The optional
screenshot run also renders each wizard page headlessly.
