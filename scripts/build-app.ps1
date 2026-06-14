# 一键本地构建（Windows）：拉取并校验 jar → jlink 最小 JRE → 构建 Tauri 安装包。
# 前置：PATH 中需有 JDK 25（jlink/java），以及 Rust 与 Node。
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $root
& "$root/scripts/fetch-kafbat-jar.ps1"
& "$root/scripts/build-jre.ps1"
npm install
npm run tauri build
Write-Host "产物位于 src-tauri/target/release/bundle/"
