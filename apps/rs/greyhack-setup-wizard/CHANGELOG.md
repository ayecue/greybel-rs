# greyhack-setup-wizard

## 0.2.0 · 2026-11-19

### Minor changes

- The setup wizard is a small Rust program, `greyhack-setup` (about 5 MB, the .NET one was about 23 MB), with a window and the same command line; `greybel setup` and the VS Code command **Greybel: Set up Grey Hack** use the same steps. It installs the message hook release of its own commit, shows the Steam launch option macOS and Linux need, and removes the duplicate `libsteam_api.dylib` that crashes the game under BepInEx on macOS. An import, build or run that can't reach the game offers the setup in VS Code. `greybel update` replaces the CLI with its newest release from DigitalOcean Spaces, and commands in a terminal mention a newer version once a day.

## 0.1.0 · 2026-11-18

- The first release of the setup wizard, a window and a command-line tool (`greyhack-setup-wizard`) that install BepInEx 5 or 6, the message hook plugin and its port in a Grey Hack installation.
- The plugin is downloaded from the repository's latest release; BepInEx archives and plugin files are checked against their SHA-256. Nothing downloads or changes before **Apply** or `--yes`.
- Self-contained programs for Windows, Linux and macOS (x64 and arm64).
