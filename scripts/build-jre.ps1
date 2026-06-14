$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$out = [System.IO.Path]::GetFullPath((Join-Path $root "../resources/jre"))
$modules = ((Get-Content (Join-Path $root "jre-modules.txt")) -join '').Trim()
$requiredMajor = 25
if (-not (Get-Command jlink -ErrorAction SilentlyContinue)) { Write-Host "::error:: 未找到 jlink，请安装 Temurin JDK $requiredMajor"; exit 1 }
$verLine = (& java -version 2>&1)[0]
Write-Host "使用的 JDK：$verLine"
if ($verLine -notmatch "version `"$requiredMajor\b") { Write-Host "::error:: 需要 JDK $requiredMajor"; exit 1 }
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
jlink --add-modules $modules --strip-debug --no-header-files --no-man-pages --compress=zip-9 --output $out
if ($LASTEXITCODE -ne 0) { Write-Host "::error:: jlink 失败，退出码 $LASTEXITCODE"; exit $LASTEXITCODE }
& "$out/bin/java.exe" -version
