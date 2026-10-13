#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

DOTNET="${DOTNET:-dotnet}"
ALL=(osx-arm64 osx-x64 linux-x64 linux-arm64 win-x64 win-arm64)

host_rid() {
    local os arch
    case "$(uname -s)" in
        Darwin*) os=osx ;;
        Linux*) os=linux ;;
        MINGW* | MSYS* | CYGWIN*) os=win ;;
        *) echo "unknown OS: $(uname -s)" >&2; exit 1 ;;
    esac
    case "$(uname -m)" in
        arm64 | aarch64) arch=arm64 ;;
        x86_64 | amd64) arch=x64 ;;
        *) echo "unknown architecture: $(uname -m)" >&2; exit 1 ;;
    esac
    echo "$os-$arch"
}

if [[ $# -eq 0 ]]; then
    rids=("$(host_rid)")
elif [[ "$1" == all ]]; then
    rids=("${ALL[@]}")
else
    rids=("$@")
fi

for rid in "${rids[@]}"; do
    echo "==> $rid"
    publish="src/obj/publish/$rid"
    "$DOTNET" publish src/MonoRegex.csproj -c Release -r "$rid" -o "$publish" --nologo -v quiet
    mkdir -p "dist/$rid"
    for library in "$publish"/{libMonoRegex.dylib,libMonoRegex.so,MonoRegex.dll}; do
        if [[ -f "$library" ]]; then
            cp "$library" "dist/$rid/"
            echo "    dist/$rid/$(basename "$library")"
        fi
    done
done
