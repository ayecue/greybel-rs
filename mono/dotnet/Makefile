# mono's .NET projects, see README.md.
#
#   mono-regex                   Native AOT regex library for the mono-regex crate
#   mono-fixtures                what the game runtime returns for the mono-number, mono-string and mono-dictionary crates

DOTNET ?= dotnet

REGEX = mono-regex
FIXTURES = mono-fixtures

.PHONY: all
all: regex

# Regex library; RIDS is a list of runtime identifiers or `all` (default: the host).

.PHONY: regex
regex:
	DOTNET="$(DOTNET)" ./$(REGEX)/build.sh $(RIDS)

# Fixtures of the mono-number, mono-string and mono-dictionary crates: runs on
# the macOS game's own Mono runtime (GREY_HACK_PATH, default Steam's location).

.PHONY: fixtures
fixtures:
	DOTNET="$(DOTNET)" ./$(FIXTURES)/run.sh

.PHONY: clean
clean:
	$(DOTNET) clean $(REGEX)/src
	$(DOTNET) clean $(FIXTURES)/src

.PHONY: help
help:
	@echo "Targets:"
	@echo "  regex [RIDS=...]       - Native regex library into $(REGEX)/dist/<rid>/ (default)"
	@echo "  fixtures               - mono-number/mono-string/mono-dictionary fixtures from the game's runtime"
	@echo "  clean                  - Clean build output"
