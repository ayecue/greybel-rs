# greybel's .NET projects, see README.md.
#
#   greyhack-message-hook        BepInEx 5/6 plugin (needs GREY_HACK_PATH), and its tests against the game's assemblies
#   greyhack-message-hook-proxy  the plugin's proxy without the game
#   greyhack-setup-wizard        the setup wizard's steps: BepInEx 5/6, the message hook and its config (its window
#                                and command-line tool are apps/dotnet's)
#   greyhack-test-support        what tests need to drive the game: `game run` (headless) and `proxy`
#   greyhack-util                the game's network contracts (../contracts) -> the duplex protocol of the
#                                messages; its world data and terminal text assets -> the assets crates' bundles

DOTNET ?= dotnet

# The Cargo workspace greyhack-util writes the assets into.
ASSETS_RUST := $(abspath ../../assets/rs)
# Downloads greyhack-util keeps (class database, font files).
UTIL_CACHE := $(abspath ../../target/greyhack-util)

MESSAGE_HOOK = greyhack-message-hook/src
MESSAGE_HOOK_PROXY = greyhack-message-hook-proxy
UTIL = $(DOTNET) run --project greyhack-util/src --

# The extracted game contract (bundle) and the plugin's own (extension.json).
CONTRACTS = ../contracts
# The duplex protocol of the messages, whose tags say what converting to the game's messages needs (game network
# fields, game message ids, pids). A game update that only adds messages is a new minor: set PROTOCOL_VERSION to it,
# and greyhack-util keeps the order of the minor before.
PROTOCOL_VERSION ?= 1.0
PROTOCOL = $(abspath ../../protocols/greyhack)/$(PROTOCOL_VERSION).protocol

.PHONY: all
all: message-hook

# Message hook plugin.

.PHONY: message-hook
message-hook: message-hook-bepinex5 message-hook-bepinex6

.PHONY: message-hook-bepinex5
message-hook-bepinex5:
	$(DOTNET) build $(MESSAGE_HOOK) --configuration Release-BepInEx5

.PHONY: message-hook-bepinex6
message-hook-bepinex6:
	$(DOTNET) build $(MESSAGE_HOOK) --configuration Release-BepInEx6

.PHONY: message-hook-debug
message-hook-debug:
	$(DOTNET) build $(MESSAGE_HOOK) --configuration Debug-BepInEx5
	$(DOTNET) build $(MESSAGE_HOOK) --configuration Debug-BepInEx6

# Network contracts: `extract` reads the installed game's Assembly-CSharp.dll,
# `generate` writes the duplex protocol of the messages. Its C# classes
# (greyhack-message-hook-proxy/src/Protocol/Protocol.g.cs) are written by
# `UPDATE_CS=1 cargo test -p greyhack-message-hook-client --test main cs::` in
# greybel/rs; the Rust client includes the protocol as it builds.

.PHONY: contracts
contracts: extract generate

# The installed game's data folder (GREY_HACK_DATA, else under GREY_HACK_PATH).
GAME_DATA = $${GREY_HACK_DATA:-$$(if [ -d "$${GREY_HACK_PATH}/Grey Hack.app" ]; then echo "$${GREY_HACK_PATH}/Grey Hack.app/Contents/Resources/Data"; else echo "$${GREY_HACK_PATH}/Grey Hack_Data"; fi)}

.PHONY: extract
extract:
	@managed="$${GREY_HACK_MANAGED_PATH:-$(GAME_DATA)/Managed}"; \
	$(UTIL) extract "$$managed/Assembly-CSharp.dll" --output $(CONTRACTS)

# Game assets: the world data and the terminal's text assets, into the assets
# crates' bundles (after a game update).
.PHONY: assets
assets:
	@$(UTIL) assets "$(GAME_DATA)" \
		--world $(ASSETS_RUST)/greyhack-world-assets \
		--text-mesh $(ASSETS_RUST)/greyhack-text-mesh-assets \
		--cache $(UTIL_CACHE)

.PHONY: generate
generate:
	@mkdir -p $(dir $(PROTOCOL))
	$(UTIL) generate --contracts $(CONTRACTS) --output $(PROTOCOL)

# greyhack-util's tests, which also fail when a generated file or, with the game
# installed, the game contract is out of date, and greyhack-test-support's. The
# proxy is tested by greyhack-message-hook-client's tests with the feature `dotnet`.
.PHONY: test
test:
	$(DOTNET) test --solution greyhack-util/GreyHackUtil.slnx
	$(DOTNET) test --solution greyhack-setup-wizard/GreyHackSetupWizard.slnx
	$(DOTNET) test --project greyhack-test-support/tests

# The plugin against the installed game's assemblies (needs GREY_HACK_PATH): the
# game's members it reaches by reflection, its Harmony patches, and every
# message written and read as the game's own reader does. The game itself is
# tested by greyhack-message-hook-client's tests with the feature `game`.
.PHONY: test-message-hook
test-message-hook:
	$(DOTNET) test --solution greyhack-message-hook/GreyHackMessageHook.slnx

.PHONY: clean
clean:
	$(DOTNET) clean $(MESSAGE_HOOK) --configuration Release-BepInEx5
	$(DOTNET) clean $(MESSAGE_HOOK) --configuration Release-BepInEx6
	$(DOTNET) clean greyhack-util/GreyHackUtil.slnx
	$(DOTNET) clean $(MESSAGE_HOOK_PROXY)/src
	$(DOTNET) clean greyhack-setup-wizard/GreyHackSetupWizard.slnx
	$(DOTNET) clean greyhack-test-support/GreyHackTestSupport.slnx

.PHONY: help
help:
	@echo "Targets:"
	@echo "  message-hook           - Release builds of the plugin for BepInEx 5 and 6 (default)"
	@echo "  message-hook-bepinex5  - GreyHackMessageHook5.dll"
	@echo "  message-hook-bepinex6  - GreyHackMessageHook.dll"
	@echo "  message-hook-debug     - Debug builds for BepInEx 5 and 6"
	@echo "  contracts              - extract + generate (after a game update)"
	@echo "  extract                - Read $(CONTRACTS)/game.bundle from the game's Assembly-CSharp.dll"
	@echo "  assets                 - Read the game's world data and terminal text assets into the assets crates' bundles"
	@echo "  generate               - Write the duplex protocol of the messages from $(CONTRACTS)"
	@echo "  test                   - greyhack-util (incl. generated files up to date), greyhack-setup-wizard and greyhack-test-support tests"
	@echo "  test-message-hook      - Plugin tests against the installed game's assemblies (needs GREY_HACK_PATH)"
	@echo "  clean                  - Clean build output"
