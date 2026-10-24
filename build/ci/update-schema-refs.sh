#!/bin/bash
#
# Points schema references in all tracked TOML files to the given release version.
#
# Matches both:
# - Relative: #:schema (../)+schema/...
# - Absolute: #:schema https://raw.githubusercontent.com/<repository>/<ref>/schema/...
#
# Usage: update-schema-refs.sh <repository> <version>

set -euo pipefail

REPOSITORY="${1:?usage: $0 <repository> <version>}"
VERSION="${2:?usage: $0 <repository> <version>}"

git grep -lz "#:schema" -- "*.toml" | while IFS= read -r -d '' file; do
    echo "Updating schema reference in ${file}"
    sed -i.bak -E 's|#:schema ((\.\./)+\|https://raw\.githubusercontent\.com/'"${REPOSITORY}"'/[^/]+/)schema/|#:schema https://raw.githubusercontent.com/'"${REPOSITORY}"'/v'"${VERSION}"'/schema/|g' "${file}"
    rm -f "${file}.bak"
done
