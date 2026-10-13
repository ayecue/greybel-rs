# The repository's checks, from its root (README.md, Development): op-lint, each language's own lint, and the
# tests. They are tasks of the reman.toml files, run by reman (tools/reman); each target here runs `reman` with the
# matching suite and filters, and REMAN_FLAGS adds more (`make test REMAN_FLAGS=--since`, `--dry-run`,
# `--no-fail-fast`). `make help` lists the targets.
#
# Unit and integration tests follow each language's own split: Rust's unit tests are those of `src/` (and the doc
# tests), its integration tests those of `tests/`, the wasm tests and the tests against the .NET proxy; the
# TypeScript workspaces' vitest suites and the .NET test projects are unit tests, the editor extension's tests in
# VS Code (desktop and web) integration tests. Tests against the installed game are `test-game` (GREY_HACK_PATH).

DOTNET ?= dotnet
REMAN ?= cargo run --quiet --manifest-path tools/reman/Cargo.toml --
REMAN_FLAGS ?=

# op-lint, run from its checkout, whose lint/ folder has the settings (lint/repositories/greybel-rs/ this
# repository's own); the op-lint task of reman.toml reads it.
OP_LINT_ROOT ?= ../op-lint
export OP_LINT_ROOT

.PHONY: all
all: lint test

# Dependencies: the TypeScript workspaces' packages and the .NET tools (CSharpier).

.PHONY: install
install:
	$(REMAN) install $(REMAN_FLAGS)

# Lints, checking only.

.PHONY: lint
lint:
	$(REMAN) lint $(REMAN_FLAGS)

# Names, the structure of the projects and the blank lines rustfmt doesn't add.
.PHONY: lint-op
lint-op:
	$(REMAN) lint --task op-lint $(REMAN_FLAGS)

# rustfmt (nightly, for rustfmt.toml's import options) and clippy in every Cargo workspace.
.PHONY: lint-rust
lint-rust:
	$(REMAN) lint --language rust $(REMAN_FLAGS)

# ESLint and Prettier in every TypeScript workspace.
.PHONY: lint-ts
lint-ts:
	$(REMAN) lint --language typescript $(REMAN_FLAGS)

# CSharpier, for every C# file but .csharpierignore's.
.PHONY: lint-cs
lint-cs:
	$(REMAN) lint --language dotnet $(REMAN_FLAGS)

# The formatters' fixes: rustfmt, then op-lint's blank lines; ESLint's fixes, then Prettier; CSharpier.

.PHONY: format
format:
	$(REMAN) format $(REMAN_FLAGS)

.PHONY: format-rust
format-rust:
	$(REMAN) format --language rust $(REMAN_FLAGS)

.PHONY: format-ts
format-ts:
	$(REMAN) format --language typescript $(REMAN_FLAGS)

.PHONY: format-cs
format-cs:
	$(REMAN) format --language dotnet $(REMAN_FLAGS)

# Tests.

.PHONY: test
test:
	$(REMAN) test unit integration $(REMAN_FLAGS)

.PHONY: test-unit
test-unit:
	$(REMAN) test unit $(REMAN_FLAGS)

# The tests of every crate's `src/`, and the doc tests (nextest runs none).
.PHONY: test-unit-rust
test-unit-rust:
	$(REMAN) test unit --language rust $(REMAN_FLAGS)

.PHONY: test-unit-ts
test-unit-ts:
	$(REMAN) test unit --language typescript $(REMAN_FLAGS)

.PHONY: test-unit-dotnet
test-unit-dotnet:
	$(REMAN) test unit --language dotnet $(REMAN_FLAGS)

.PHONY: test-integration
test-integration:
	$(REMAN) test integration $(REMAN_FLAGS)

# Every crate's `tests/`; greyscript-intrinsics' wasm tests (wasm-bindgen-test-runner); the message hook client
# against the plugin's proxy (the .NET SDK).
.PHONY: test-integration-rust
test-integration-rust:
	$(REMAN) test integration --language rust $(REMAN_FLAGS)

# The editor extension in VS Code, desktop and web.
.PHONY: test-integration-ts
test-integration-ts:
	$(REMAN) test integration --language typescript $(REMAN_FLAGS)

# Against the installed game (GREY_HACK_PATH; BEPINEX=6 for BepInEx 6), headless: the parity of greybel's emulation
# with the game (greyhack-test-support-client), the message hook client, the runtime and the builder in the game, and
# the plugin against the game's assemblies.
.PHONY: test-game
test-game:
	$(REMAN) test game $(REMAN_FLAGS)

# The setup wizard's window.
.PHONY: setup-wizard
setup-wizard:
	$(DOTNET) run --project apps/dotnet/greyhack-setup-wizard/gui

.PHONY: help
help:
	@echo "Targets (each runs reman: tools/reman; REMAN_FLAGS=--since, --dry-run, --no-fail-fast):"
	@echo "  all                    - lint, then test (default)"
	@echo "  install                - pnpm install in every TypeScript workspace, dotnet tool restore"
	@echo "  lint                   - lint-op, lint-rust, lint-ts and lint-cs, checking only"
	@echo "  lint-op                - op-lint: names, structure, padding ($(OP_LINT_ROOT))"
	@echo "  lint-rust              - rustfmt --check (nightly) and clippy -D warnings, every Cargo workspace"
	@echo "  lint-ts                - ESLint and Prettier --check, every TypeScript workspace"
	@echo "  lint-cs                - CSharpier check"
	@echo "  format                 - format-rust, format-ts and format-cs: the formatters' fixes"
	@echo "  format-rust            - rustfmt (nightly), then op-lint padding --fix"
	@echo "  format-ts              - ESLint --fix, then Prettier"
	@echo "  format-cs              - CSharpier format"
	@echo "  test                   - test-unit, then test-integration"
	@echo "  test-unit              - Rust src/ and doc tests, the TypeScript workspaces' tests, the .NET test projects"
	@echo "  test-integration       - Rust tests/, wasm and proxy tests; the editor extension in VS Code"
	@echo "  test-game              - tests against the installed game (needs GREY_HACK_PATH)"
	@echo "  setup-wizard           - open the setup wizard's window"
