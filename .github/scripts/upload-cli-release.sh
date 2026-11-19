#!/usr/bin/env bash
# Publishes a greybel release for `greybel update` on DigitalOcean Spaces: its archives, from a folder with the
# release's files, as releases/greybel/<version>/<archive>, then releases/greybel/latest.json naming the version and
# each Rust target's archive with its SHA-256 and size. Archives are immutable; latest.json is not cached.
# Usage: upload-cli-release.sh <version> <tag> <folder>. Needs SPACES_BUCKET, SPACES_ENDPOINT and
# AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY (the Spaces key).
set -euo pipefail

: "${SPACES_BUCKET:?}" "${SPACES_ENDPOINT:?}"

version=$1
tag=$2
folder=$3
prefix="releases/greybel"
public_url="https://$SPACES_BUCKET.${SPACES_ENDPOINT#https://}"
assets='{}'

put() {
  local key=$1 file=$2 type=$3 cache=$4

  aws s3api put-object \
    --bucket "$SPACES_BUCKET" \
    --key "$key" \
    --body "$file" \
    --acl public-read \
    --content-type "$type" \
    --cache-control "$cache" \
    --endpoint-url "$SPACES_ENDPOINT" > /dev/null
}

shopt -s nullglob
for file in "$folder/greybel-$version-"*.tar.gz "$folder/greybel-$version-"*.zip; do
  name=$(basename "$file")
  target=${name#"greybel-$version-"}
  target=${target%.tar.gz}
  target=${target%.zip}
  key="$prefix/$version/$name"
  type=application/gzip
  if [[ $name == *.zip ]]; then
    type=application/zip
  fi

  put "$key" "$file" "$type" 'public, max-age=31536000, immutable'
  assets=$(jq --arg target "$target" --arg url "$public_url/$key" \
    --arg sha256 "$(sha256sum "$file" | cut -d ' ' -f 1)" --argjson size "$(wc -c < "$file")" \
    '. + {($target): {url: $url, sha256: $sha256, size: $size}}' <<< "$assets")
  echo "uploaded: $key"
done

if [ "$assets" = '{}' ]; then
  echo "no greybel-$version-* archives in $folder" >&2
  exit 1
fi

jq -n --arg version "$version" --arg tag "$tag" --argjson assets "$assets" \
  '{version: $version, tag: $tag, assets: $assets}' > "$folder/latest.json"
put "$prefix/latest.json" "$folder/latest.json" application/json 'no-cache'
rm "$folder/latest.json"
echo "uploaded: $prefix/latest.json ($version)"
