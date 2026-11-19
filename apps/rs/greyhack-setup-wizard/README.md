# greyhack-setup-wizard

`greyhack-setup`: installs BepInEx, the Grey Hack message hook and its port.
Without arguments it opens a window; with a command it runs on the command
line, like `greybel setup`. The steps are the
[`greyhack-setup` crate](../../../greybel/rs/greyhack-setup/README.md); see the
[Grey Hack integration guide](../../../docs/greyhack-integration.md#set-up).

```sh
cargo run -p greyhack-setup-wizard                        # the window
cargo run -p greyhack-setup-wizard -- status              # the command line
cargo run -p greyhack-setup-wizard --no-default-features  # without the window, as in the VS Code extension
```

The window shows the game's setup, offers BepInEx 5 or 6 (or a zip file), the
message hook (its release or a file) and the port, and lists every download and
change for review before **Apply**. On Windows the window's build has no
console; use `greybel setup` for the command line there.

```sh
cargo nextest run -p greyhack-setup-wizard
```

The tests drive the window through its accessibility tree against fake game
folders.
