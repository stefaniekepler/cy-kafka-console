#!/usr/bin/env node
// 合并各平台的 updater 元数据片段（updater-*.json），生成 Tauri 动态更新清单 latest.json。
// 同时删除中间片段文件，避免被一并发布。
//
// 入参（环境变量）：
//   DIR                收集目录（含 updater-*.json 与重命名后的工件）
//   VERSION            版本号（如 0.1.0，不带 v）
//   GITHUB_REPOSITORY  owner/repo（GitHub Actions 自带）
//   TAG                发布 tag（如 v0.1.0），用于拼下载 URL
//   NOTES              发布说明（可选）
import { readdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs'
import { join } from 'node:path'

const env = (k) => {
  const v = process.env[k]
  if (!v) { console.error(`缺少环境变量 ${k}`); process.exit(1) }
  return v
}
const DIR = env('DIR')
const VERSION = env('VERSION')
const REPO = env('GITHUB_REPOSITORY')
const TAG = env('TAG')
const NOTES = process.env.NOTES || `Release ${TAG}`

const baseUrl = `https://github.com/${REPO}/releases/download/${TAG}`
const platforms = {}

for (const name of readdirSync(DIR)) {
  if (!name.startsWith('updater-') || !name.endsWith('.json')) continue
  const meta = JSON.parse(readFileSync(join(DIR, name), 'utf8'))
  platforms[meta.platform] = {
    signature: meta.signature,
    url: `${baseUrl}/${encodeURIComponent(meta.file)}`,
  }
  rmSync(join(DIR, name)) // 中间片段不发布
}

if (Object.keys(platforms).length === 0) {
  console.error('::error:: 未收集到任何平台的 updater 元数据，latest.json 不生成')
  process.exit(1)
}

const manifest = {
  version: VERSION,
  notes: NOTES,
  pub_date: new Date().toISOString().replace(/\.\d+Z$/, 'Z'),
  platforms,
}
writeFileSync(join(DIR, 'latest.json'), JSON.stringify(manifest, null, 2))
console.log('生成 latest.json：')
console.log(JSON.stringify(manifest, null, 2))
