$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Get-Content "$root/kafbat.env" | ForEach-Object {
  if ($_ -match '^\s*([^#=]+)=(.*)$') { Set-Variable -Name $Matches[1].Trim() -Value $Matches[2].Trim() }
}
$destDir = Join-Path $root "../resources/kafbat"
New-Item -ItemType Directory -Force -Path $destDir | Out-Null
$dest = Join-Path $destDir $KAFBAT_JAR
Write-Host "下载 $KAFBAT_URL"
Invoke-WebRequest -Uri $KAFBAT_URL -OutFile $dest
$actual = (Get-FileHash -Algorithm SHA256 $dest).Hash.ToLower()
Write-Host "实际 SHA-256: $actual"
if ($KAFBAT_SHA256 -eq "REPLACE_WITH_VERIFIED_SHA256") {
  Write-Host "::warning:: 尚未固化校验和，请将 $actual 填入 scripts/kafbat.env"
} elseif ($actual -ne $KAFBAT_SHA256.ToLower()) {
  Write-Host "::error:: SHA-256 不匹配！期望 $KAFBAT_SHA256 实际 $actual"; exit 1
} else { Write-Host "SHA-256 校验通过" }
