#!/usr/bin/env node
// 将 Tauri 构建产物重命名为「体现系统版本」的统一命名，收集到 OUT_DIR，
// 并为该平台输出 updater 元数据片段 OUT_DIR/updater-<platform>.json，供后续合成 latest.json。
//
// 命名格式：<SLUG>_<VERSION>_<SYS_LABEL>_<ARCH_LABEL><后缀>
//   例：Kafka-Console_0.1.0_macos14_x64.dmg / Kafka-Console_0.1.0_win11_x64-setup.exe
//
// 入参（环境变量）：
//   BUNDLE_DIR  src-tauri/target/<target>/release/bundle
//   OUT_DIR     收集目录（如 dist-release）
//   SLUG        产品名 slug（如 Kafka-Console，避免空格）
//   VERSION     版本号（如 0.1.0，不带 v）
//   SYS_LABEL   系统版本标签：macos14 | win11 | linux
//   ARCH_LABEL  架构标签：x64 | aarch64 | amd64
//   PLATFORM    updater 平台键：darwin-x86_64 | darwin-aarch64 | windows-x86_64 | linux-x86_64
import { mkdirSync, copyFileSync, readFileSync, writeFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

const env = (k) => {
  const v = process.env[k]
  if (!v) { console.error(`缺少环境变量 ${k}`); process.exit(1) }
  return v
}
const BUNDLE_DIR = env('BUNDLE_DIR')
const OUT_DIR = env('OUT_DIR')
const SLUG = env('SLUG')
const VERSION = env('VERSION')
const SYS_LABEL = env('SYS_LABEL')
const ARCH_LABEL = env('ARCH_LABEL')
const PLATFORM = env('PLATFORM')

function walk(dir) {
  const out = []
  for (const name of readdirSync(dir)) {
    const p = join(dir, name)
    if (statSync(p).isDirectory()) out.push(...walk(p))
    else out.push(p)
  }
  return out
}

const base = `${SLUG}_${VERSION}_${SYS_LABEL}_${ARCH_LABEL}`

// 识别的产物后缀（.sig 变体单列，匹配互斥，顺序无关）。
const suffixes = [
  '.app.tar.gz.sig', '.app.tar.gz',
  '-setup.exe.sig', '-setup.exe',
  '.AppImage.sig', '.AppImage',
  '.dmg', '.deb',
]

mkdirSync(OUT_DIR, { recursive: true })

let files
try {
  files = walk(BUNDLE_DIR)
} catch (e) {
  console.error(`无法读取 bundle 目录 ${BUNDLE_DIR}: ${e.message}`)
  process.exit(1)
}

const renamed = {} // 后缀 -> 新文件名
for (const f of files) {
  for (const suf of suffixes) {
    if (f.endsWith(suf)) {
      const newName = `${base}${suf}`
      copyFileSync(f, join(OUT_DIR, newName))
      renamed[suf] = newName
      console.log(`重命名 ${f} -> ${OUT_DIR}/${newName}`)
      break
    }
  }
}

// 该平台对应的 updater 工件与其签名（mac=.app.tar.gz，win=-setup.exe，linux=.AppImage）。
const updaterSuffix = PLATFORM.startsWith('darwin') ? '.app.tar.gz'
  : PLATFORM.startsWith('windows') ? '-setup.exe'
    : '.AppImage'
const updaterFile = renamed[updaterSuffix]
const sigFile = renamed[`${updaterSuffix}.sig`]

if (updaterFile && sigFile) {
  const signature = readFileSync(join(OUT_DIR, sigFile), 'utf8').trim()
  const meta = { platform: PLATFORM, file: updaterFile, signature }
  writeFileSync(join(OUT_DIR, `updater-${PLATFORM}.json`), JSON.stringify(meta, null, 2))
  console.log(`updater 元数据 -> updater-${PLATFORM}.json (${updaterFile})`)
} else {
  console.warn(`::warning:: 平台 ${PLATFORM} 缺少 updater 工件或签名 `
    + `(updater=${updaterFile ?? '无'}, sig=${sigFile ?? '无'})，该平台不会进入 latest.json`)
}
