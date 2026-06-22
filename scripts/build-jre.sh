#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$SCRIPT_DIR/../resources/jre"
MODULES=$(cat "$SCRIPT_DIR/jre-modules.txt")
REQUIRED_MAJOR=25

# 若 PATH 中的 java 不是 JDK 25（如本机默认 JDK 8），自动在本机定位 JDK 25 并导出 JAVA_HOME/PATH。
# CI 中 PATH 已是 JDK 25 时此步直接跳过，行为不变。
JDK_REQUIRED_MAJOR="$REQUIRED_MAJOR" source "$SCRIPT_DIR/jdk-env.sh"

command -v jlink >/dev/null || { echo "::error:: 未找到 jlink，请安装 Temurin JDK ${REQUIRED_MAJOR}" >&2; exit 1; }

# 断言 JDK 大版本匹配（设计要求：CI 断言 java -version 匹配）
VER_LINE=$(java -version 2>&1 | head -1)
echo "使用的 JDK：$VER_LINE"
MAJOR=$(echo "$VER_LINE" | sed -E 's/.*version "([0-9]+).*/\1/')
if [ "$MAJOR" != "$REQUIRED_MAJOR" ]; then
  echo "::error:: 需要 JDK ${REQUIRED_MAJOR}，当前为 ${MAJOR}" >&2; exit 1
fi

rm -rf "$OUT"
jlink --add-modules "$MODULES" \
  --strip-debug --no-header-files --no-man-pages --compress=zip-9 \
  --output "$OUT"

echo "jlink 运行时大小：$(du -sh "$OUT" | awk '{print $1}')"
"$OUT/bin/java" -version
