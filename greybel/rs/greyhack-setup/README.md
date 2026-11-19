# greyhack-setup

Sets up Grey Hack for the message hook: finds the game, installs BepInEx 5 or 6
from pinned archives, installs the message hook from its GitHub release or a
file, and sets the hook's port. greyhack-setup-wizard's window and command
line, `greybel setup` and the VS Code extension use it.

| Module | Contents |
|---|---|
| `game` | `GameLocator` (`GREY_HACK_PATH`, then the Steam libraries with app 605230), `GameLayout` |
| `bepinex` | Pinned `BepInExRelease`s with SHA-256, `BepInExInstaller`, `BepInExInstallation`, `BepInExLauncher` (`run_bepinex.sh`) |
| `hook` | `HookRelease` (GitHub releases, checked against their digest), `HookInstaller`, `HookConfig` (`General.Port`) |
| `Setup`, `GameStatus` | The steps for one game and what it has of the setup |
| `cli` (feature `cli`) | The `setup` command line, as text or JSON lines |

Pinned versions: BepInEx 5.4.23.6 and 6.0.0-be.791. Replacing or removing
BepInEx changes neither `BepInEx/plugins` nor `BepInEx/config`. BepInEx 6 for
macOS is x64 only, so its installation makes `run_bepinex.sh` run the game
under Rosetta. `MessageHook::VERSION` is the message hook version of the same
commit (read from its project by `build.rs`), the release `hook install`
downloads by default.

```sh
cargo nextest run -p greyhack-setup --all-features
```

The tests use fake game folders, zip archives and a local HTTP server.
