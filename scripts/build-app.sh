#!/usr/bin/env bash
# 一键本地构建：拉取并校验 kafbat jar → jlink 最小 JRE → 构建 Tauri 安装包。
# 前置：PATH 中需有 JDK 25（jlink/java），以及 Rust 与 Node。
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."
./scripts/fetch-kafbat-jar.sh
./scripts/build-jre.sh
npm install
npm run tauri build
echo "产物位于 src-tauri/target/release/bundle/"
