---
rs/greyhack-setup: minor
rs/greyhack-setup-wizard: minor
rs/greybel-cli: minor
ts/greybel-vs: minor
---

The setup wizard is a small Rust program, `greyhack-setup` (about 5 MB, the .NET one was about 23 MB), with a window and the same command line; `greybel setup` and the VS Code command **Greybel: Set up Grey Hack** use the same steps. It installs the message hook release of its own commit, shows the Steam launch option macOS and Linux need, and removes the duplicate `libsteam_api.dylib` that crashes the game under BepInEx on macOS. An import, build or run that can't reach the game offers the setup in VS Code. `greybel update` replaces the CLI with its newest release from DigitalOcean Spaces, and commands in a terminal mention a newer version once a day.
