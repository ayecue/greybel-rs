# Grey Hack integration

Greybel talks to a running Grey Hack single-player game through the
`greyhack-message-hook` BepInEx plugin. The setup wizard installs BepInEx and
the plugin; the message proxy carries builds, imports, script contexts and the
debugger between the game and Greybel's CLI or editor.

For automated, isolated game sessions rather than your own game, see
[Game testing](game-testing.md).

## Prerequisites

- Grey Hack, installed through Steam.
- The .NET 10 SDK to run the setup wizard from this repository or build the
  plugin.
- The `greybel` CLI for the command examples below.
- A plugin DLL from a Greybel release, or a locally built plugin matching the
  BepInEx major you install.

The wizard finds a game specified by `--game`, then `GREY_HACK_PATH`, then the
Steam libraries containing app 605230. The game folder is the one containing
`Grey Hack.app`, `Grey Hack.exe`, or the Linux executable and its data folder.

## Set up with the wizard

From the repository root, open the graphical wizard:

```sh
make setup-wizard
# Equivalent:
dotnet run --project apps/dotnet/greyhack-setup-wizard/gui
```

Check the detected game and its status, then choose **Set up…**. The wizard
lets you install or remove BepInEx, select the matching message-hook build,
and set the loopback TCP port (7777 by default). Review the summary and choose
**Apply**; downloads and file changes do not start before that confirmation.
A port-in-use report is a warning, not a blocker.

The equivalent CLI is in `apps/dotnet/greyhack-setup-wizard`. From that folder,
a typical BepInEx 5 installation from a published plugin release is:

```sh
dotnet run --project cli -- locate --game "/path/to/Grey Hack"
dotnet run --project cli -- bepinex install --bepinex 5 --yes --game "/path/to/Grey Hack"
dotnet run --project cli -- plugin install --release latest --bepinex 5 --yes --game "/path/to/Grey Hack"
dotnet run --project cli -- configure --port 7777 --game "/path/to/Grey Hack"
```

Omit `--yes` to confirm each download interactively. A non-interactive command
does not download without `--yes`. To install a local build instead, replace
the `plugin install` command with:

```sh
dotnet run --project cli -- plugin install --assembly "/path/to/GreyHackMessageHook5.dll" --bepinex 5 --game "/path/to/Grey Hack"
```

The wizard verifies pinned BepInEx downloads against their recorded SHA-256
hashes, and release plugin downloads against GitHub's asset digest. It keeps
`BepInEx/plugins` and `BepInEx/config` when replacing the BepInEx loader and
core files. See the [wizard README](../apps/dotnet/greyhack-setup-wizard/README.md)
for every command and the [setup library README](../greybel/dotnet/greyhack-setup-wizard/README.md)
for installation details and pinned versions.

### Build the plugin yourself

Set `GREY_HACK_PATH` to the game folder, or `GREY_HACK_MANAGED_PATH` directly to
its `Managed` folder, then run from `greybel/dotnet`:

```sh
make message-hook             # Release builds for BepInEx 5 and 6
make message-hook-bepinex5   # Only GreyHackMessageHook5.dll
make message-hook-bepinex6   # Only GreyHackMessageHook.dll
make message-hook-debug      # Debug builds of both
```

Install the resulting DLL with `plugin install --assembly`, or copy only that
DLL into `BepInEx/plugins`. Do not add another `Newtonsoft.Json.dll`: the game
already supplies it. The output paths and test commands are in the
[message-hook README](../greybel/dotnet/greyhack-message-hook/README.md).

BepInEx 5 is the stable line; the wizard currently pins 5.4.23.6. BepInEx 6 is
a prerelease line, currently pinned to 6.0.0-be.791. Its macOS build is x64
only, so on Apple Silicon the wizard configures `run_bepinex.sh` to run the
game under Rosetta.

## Start the game and connect

Start Grey Hack with BepInEx enabled and enter single player. On macOS and
Linux, run `run_bepinex.sh` with the game executable as its argument from the
game folder, for example:

```sh
./run_bepinex.sh "Grey Hack.app"       # macOS
./run_bepinex.sh "Grey Hack.x86_64"   # Linux; use the installed executable's name
```

The plugin starts its listener when the BIOS menu starts. Wait until the
player and desktop are available before importing, building or running. The
listener is bound to `127.0.0.1`, not a public interface. Its BepInEx setting
is `Port` in `BepInEx/config/GreyHackMessageHook.cfg`; pass the same value to
Greybel with `--port`.

## Import, build and run in the game

These commands use the current player's computer:

```sh
# Copy a local file or folder tree into the game without transpiling it.
greybel import main.src --ingame-directory /root/project --port 7777

# Build locally and create the deployment in Grey Hack.
greybel build main.src --create-ingame --ingame-directory /root/project --port 7777

# Compile during deployment, remove the deployed sources, and permit import_code at runtime.
greybel build main.src --create-ingame --ingame-directory /root/project \
  --auto-compile --allow-import --port 7777

# Build and launch a program in the live game.
greybel execute main.src --env-type in-game --port 7777

# Pass program parameters, or run with the terminal debugger.
greybel execute main.src --env-type in-game --port 7777 -p first second
greybel execute main.src --env-type in-game --port 7777 --debug --break main.src:10
```

`greybel build --create-ingame` also writes the ordinary local build output.
`--auto-compile` applies to the in-game deployment; use `--allow-import` only
when the compiled program needs runtime `import_code`. `greybel execute`
creates a script context through the hook and supports output, input,
breakpoints, stepping and evaluation. Project defaults in `greybel.json` and
`--profile` apply to these commands as they do to local builds and runs.

## Message proxy and wire format

The plugin contains the game-independent `greyhack-message-hook-proxy` and the
duplex core. Each TCP client gets a duplex channel using
`protocols/greyhack`:

```text
Greybel -> duplex channel -> MessageProxy -> Converter -> NetworkMessages.RpcMessage -> Grey Hack
Greybel <- broadcast to all channels <- MessageProxy <- Converter <- game notification
```

Client messages go to the current player client or to plugin build/context
handlers. Game notifications are broadcast, so clients correlate events by
fields such as `build_id` and `context_id` rather than assuming a response is
theirs.

The game's JSON wire message is an object with a numeric `method` and an
ordered `params` array:

```json
{"method":1011,"params":["context-id",1]}
```

It is not an `ID` plus `strValues`/`intValues` collections. Parameter order and
CLR types come from `NetworkMessages.RpcMessage.GetParam<T>()` calls in the
receiver, with constructor arguments supplying layouts for messages whose
receivers do not define one. Optional trailing parameters may be omitted, and
nullable parameters may be explicit JSON nulls. Conversion maps both an absent
optional and an explicit null to null rather than preserving distinct values.
Struct values are JSON objects using their game field names.

`RawMessage` represents that JSON in .NET. `Converter.Encode` and
`Converter.Decode(raw, sender)` translate between it and generated duplex
messages. `IsExtension` distinguishes plugin messages, and `GetWindowPids`
reads fields tagged `pid`. Game IDs are direction-specific: `Role.Client` is a
message to the game server and `Role.Provider` one to its client.

The contracts and generated protocol are maintained by
[`greyhack-util`](../greybel/dotnet/greyhack-util/README.md); the standalone
[proxy README](../greybel/dotnet/greyhack-message-hook-proxy/README.md) covers
its API and tests.

## Troubleshooting

| Symptom | Check |
|---|---|
| Connection refused or repeated connect attempts | Confirm the game reached the BIOS/single-player menu, BepInEx loaded `GreyHackMessageHook`, and the CLI/editor port matches `GreyHackMessageHook.cfg`. Only one process can listen on that loopback port. |
| Connected but no player, or a build cannot open its windows | Enter single player and wait for the desktop to finish booting. Reconnect after returning to the menu. |
| Plugin missing or immediately fails | Match `GreyHackMessageHook5.dll` to BepInEx 5 and `GreyHackMessageHook.dll` to BepInEx 6. Remove the other major's hook build and any duplicate Newtonsoft.Json copied manually. Inspect BepInEx's `LogOutput.log`. |
| Game-version or missing-member warning | Re-extract contracts and rebuild against the installed game. Reflection/Harmony changes may also require plugin changes; run `make test-message-hook`. |
| BepInEx 6 does not start on Apple Silicon | Use the wizard's current BepInEx 6 installation and Rosetta-capable launcher, or choose BepInEx 5. |
| A download was declined or no terminal is available | Confirm at the prompt or pass `--yes`; alternatively use `bepinex install --zip` or `plugin install --assembly`. |

After a game update, the usual regeneration sequence is `make contracts` and
`make message-hook` in `greybel/dotnet`, followed by the C# binding regeneration
documented in the [message-hook README](../greybel/dotnet/greyhack-message-hook/README.md#protocol-and-updates).
