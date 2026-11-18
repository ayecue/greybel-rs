#!/usr/bin/env bash
# Uploads the .wasm files of the built sites in the given dist folders to DigitalOcean Spaces, brotli-compressed, and
# removes them from the folders, so Netlify doesn't serve them. Vite names them by content hash: an existing object
# is the same file and is skipped, and old objects stay for pages built before.
# Needs SPACES_BUCKET, SPACES_ENDPOINT, SPACES_FOLDER and AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY (the Spaces key).
set -euo pipefail

: "${SPACES_BUCKET:?}" "${SPACES_ENDPOINT:?}" "${SPACES_FOLDER:?}"

for dist in "$@"; do
  while IFS= read -r -d '' file; do
    key="$SPACES_FOLDER/$(basename "$file")"

    if aws s3api head-object --bucket "$SPACES_BUCKET" --key "$key" --endpoint-url "$SPACES_ENDPOINT" > /dev/null 2>&1; then
      echo "present: $key"
    else
      brotli --quality=11 --force --output="$file.br" "$file"
      aws s3api put-object \
        --bucket "$SPACES_BUCKET" \
        --key "$key" \
        --body "$file.br" \
        --acl public-read \
        --content-type application/wasm \
        --content-encoding br \
        --cache-control 'public, max-age=31536000, immutable' \
        --endpoint-url "$SPACES_ENDPOINT" > /dev/null
      echo "uploaded: $key ($(wc -c < "$file") bytes, $(wc -c < "$file.br") compressed)"
      rm "$file.br"
    fi

    rm "$file"
  done < <(find "$dist" -name '*.wasm' -print0)
done
