#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/kafbat.env"
DEST_DIR="$SCRIPT_DIR/../resources/kafbat"
DEST="$DEST_DIR/$KAFBAT_JAR"
mkdir -p "$DEST_DIR"

# sha256 helper: prefer shasum (macOS / perl), fall back to sha256sum (Linux coreutils)
sha256() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    sha256sum "$1" | awk '{print $1}'
  fi
}

echo "下载 $KAFBAT_URL"
curl -fSL "$KAFBAT_URL" -o "$DEST"

ACTUAL=$(sha256 "$DEST")
echo "实际 SHA-256: $ACTUAL"
if [ "$KAFBAT_SHA256" = "REPLACE_WITH_VERIFIED_SHA256" ]; then
  echo "::warning:: 尚未固化校验和，请将 $ACTUAL 填入 scripts/kafbat.env" >&2
elif [ "$ACTUAL" != "$KAFBAT_SHA256" ]; then
  echo "::error:: SHA-256 不匹配！期望 $KAFBAT_SHA256 实际 $ACTUAL" >&2
  exit 1
else
  echo "SHA-256 校验通过"
fi
