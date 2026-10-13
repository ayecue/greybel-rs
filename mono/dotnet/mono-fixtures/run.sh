#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
game="${GREY_HACK_PATH:-$HOME/Library/Application Support/Steam/steamapps/common/Grey Hack}"
contents="$game/Grey Hack.app/Contents"
if [ ! -d "$contents" ]; then
    echo "run.sh: the macOS game is not at $game (set GREY_HACK_PATH)" >&2
    exit 1
fi

"${DOTNET:-dotnet}" build "$here/src/MonoFixtures.csproj" --configuration Release --nologo --verbosity quiet
program="$here/src/bin/Release/net472/MonoFixtures.exe"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
"${CC:-cc}" -O2 -o "$work/game-mono" "$here/host/game-mono.c"

mkdir "$work/managed"
for assembly in mscorlib System System.Core; do
    cp "$contents/Resources/Data/Managed/$assembly.dll" "$work/managed/"
done
ln -s "$contents/Frameworks" "$work/lib"

"$work/game-mono" "$contents/Frameworks/libmonobdwgc-2.0.dylib" "$work/managed" \
    "$contents/MonoBleedingEdge/etc" "$program"
