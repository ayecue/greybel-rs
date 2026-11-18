#!/usr/bin/env bash

set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)

if [ -f "$root/.env.local" ]; then
  set -a
  # shellcheck source=/dev/null
  . "$root/.env.local"
  set +a
fi

: "${SPACES_KEY:?}" "${SPACES_SECRET:?}" "${GREY_HACK_ASSEMBLY_KEY:?}"

assembly=${1:-}

if [ -z "$assembly" ]; then
  if [ -n "${GREY_HACK_MANAGED_PATH:-}" ]; then
    managed=$GREY_HACK_MANAGED_PATH
  elif [ -d "${GREY_HACK_PATH:?set GREY_HACK_PATH or pass the assembly}/Grey Hack.app" ]; then
    managed="$GREY_HACK_PATH/Grey Hack.app/Contents/Resources/Data/Managed"
  else
    managed="$GREY_HACK_PATH/Grey Hack_Data/Managed"
  fi

  assembly="$managed/Assembly-CSharp.dll"
fi

url=https://greybel-assets.fra1.digitaloceanspaces.com/ci/message-hook-assembly.enc
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

openssl enc -aes-256-cbc -pbkdf2 -iter 600000 -salt -pass env:GREY_HACK_ASSEMBLY_KEY \
  -in "$assembly" -out "$work/upload.enc"
curl --fail --silent --show-error \
  --aws-sigv4 'aws:amz:us-east-1:s3' --user "$SPACES_KEY:$SPACES_SECRET" \
  --header 'x-amz-acl: public-read' --header 'Content-Type: application/octet-stream' \
  --upload-file "$work/upload.enc" "$url"

curl --fail --silent --show-error --output "$work/download.enc" "$url"
openssl enc -d -aes-256-cbc -pbkdf2 -iter 600000 -pass env:GREY_HACK_ASSEMBLY_KEY \
  -in "$work/download.enc" -out "$work/Assembly-CSharp.dll"

if cmp --silent "$assembly" "$work/Assembly-CSharp.dll"; then
  echo "uploaded $url: $assembly ($(wc -c < "$assembly" | tr -d ' ') bytes)"
else
  echo "$url doesn't decrypt to $assembly" >&2
  exit 1
fi
