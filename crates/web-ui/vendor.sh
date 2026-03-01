#!/usr/bin/env bash
# Download and verify vendored JS dependencies for assistant-web-ui.
#
# Reads crates/web-ui/vendor.lock for URLs and SHA-256 integrity hashes.
# Files are placed in src/static_assets/vendor/ and gitignored.
#
# Usage:
#   make vendor          (from repo root)
#   ./vendor.sh          (from crates/web-ui/)
#   ./vendor.sh --check  (verify existing files without downloading)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOCK_FILE="$SCRIPT_DIR/vendor.lock"
VENDOR_DIR="$SCRIPT_DIR/src/static_assets/vendor"
CHECK_ONLY=false

if [[ "${1:-}" == "--check" ]]; then
    CHECK_ONLY=true
fi

if ! command -v jq &>/dev/null; then
    echo "error: jq is required but not installed" >&2
    exit 1
fi

if [[ ! -f "$LOCK_FILE" ]]; then
    echo "error: vendor.lock not found at $LOCK_FILE" >&2
    exit 1
fi

mkdir -p "$VENDOR_DIR"

PACKAGES=$(jq -c '.packages[]' "$LOCK_FILE")
FAILED=0

while IFS= read -r pkg; do
    NAME=$(echo "$pkg" | jq -r '.name')
    VERSION=$(echo "$pkg" | jq -r '.version')
    FILE=$(echo "$pkg" | jq -r '.file')
    URL=$(echo "$pkg" | jq -r '.url')
    EXPECTED_HASH=$(echo "$pkg" | jq -r '.sha256')
    DEST="$VENDOR_DIR/$FILE"

    if $CHECK_ONLY; then
        if [[ ! -f "$DEST" ]]; then
            echo "MISSING  $NAME@$VERSION ($FILE)"
            FAILED=1
            continue
        fi
        ACTUAL_HASH=$(shasum -a 256 "$DEST" | cut -d' ' -f1)
        if [[ "$ACTUAL_HASH" != "$EXPECTED_HASH" ]]; then
            echo "MISMATCH $NAME@$VERSION ($FILE)"
            echo "  expected: $EXPECTED_HASH"
            echo "  actual:   $ACTUAL_HASH"
            FAILED=1
        else
            echo "OK       $NAME@$VERSION ($FILE)"
        fi
        continue
    fi

    # Skip download if file exists and hash matches.
    if [[ -f "$DEST" ]]; then
        ACTUAL_HASH=$(shasum -a 256 "$DEST" | cut -d' ' -f1)
        if [[ "$ACTUAL_HASH" == "$EXPECTED_HASH" ]]; then
            echo "ok       $NAME@$VERSION (cached)"
            continue
        fi
        echo "stale    $NAME@$VERSION (hash mismatch, re-downloading)"
    fi

    echo "fetch    $NAME@$VERSION"
    curl -sL --fail "$URL" -o "$DEST.tmp"

    ACTUAL_HASH=$(shasum -a 256 "$DEST.tmp" | cut -d' ' -f1)
    if [[ "$ACTUAL_HASH" != "$EXPECTED_HASH" ]]; then
        rm -f "$DEST.tmp"
        echo "error: integrity check failed for $NAME@$VERSION" >&2
        echo "  expected: $EXPECTED_HASH" >&2
        echo "  actual:   $ACTUAL_HASH" >&2
        FAILED=1
        continue
    fi

    mv "$DEST.tmp" "$DEST"
    echo "ok       $NAME@$VERSION"
done <<< "$PACKAGES"

if [[ $FAILED -ne 0 ]]; then
    echo ""
    if $CHECK_ONLY; then
        echo "Vendor check failed. Run 'make vendor' to download dependencies."
    else
        echo "Some vendor downloads failed."
    fi
    exit 1
fi

echo ""
echo "All vendor dependencies are up to date."
