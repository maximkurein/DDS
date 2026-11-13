# Сборка DDS Studio для Windows: нативное приложение на egui (crates/dds-egui).
#
#   .\build_windows.ps1                  # DDS-Studio-vX.Y.Z-windows-x64.zip (DDS Studio.exe)
#   .\build_windows.ps1 -WithServer      # плюс консольный dds-server.exe (веб-режим)
#
# Версия берётся из config\version.json. Результат: release\vX.Y.Z\ + checksum.txt (SHA-256).
# Требования: Rust (rustup, toolchain MSVC), Visual Studio Build Tools (C++). Node.js не нужен.

[CmdletBinding()]
param(
    [switch]$WithServer,
    [switch]$SkipVersionCheck,
    [switch]$Open
)

$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot
Set-Location $Root
$Slug = "DDS-Studio"

function Info($m) { Write-Host "> $m" -ForegroundColor Cyan }
function Ok($m)   { Write-Host "OK $m" -ForegroundColor Green }
function Fail($m) { Write-Host "ERROR: $m" -ForegroundColor Red; exit 1 }
function Need($tool, $hint) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { Fail "не найден '$tool' — $hint" }
}
# Get-FileHash есть не всегда: если PSModulePath перебит (например, PowerShell 5.1 запущен
# из окружения PowerShell 7), командлет не находится — считаем SHA-256 средствами .NET.
function FileSha256([string]$path) {
    if (Get-Command Get-FileHash -ErrorAction SilentlyContinue) {
        return (Get-FileHash $path -Algorithm SHA256).Hash.ToLower()
    }
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $fs = [System.IO.File]::OpenRead($path)
        try { return [BitConverter]::ToString($sha.ComputeHash($fs)).Replace("-", "").ToLower() }
        finally { $fs.Dispose() }
    }
    finally { $sha.Dispose() }
}
function Run([string]$exe, [string[]]$argv) {
    Write-Host "  `$ $exe $($argv -join ' ')" -ForegroundColor DarkGray
    & $exe @argv
    if ($LASTEXITCODE -ne 0) { Fail "команда завершилась с кодом ${LASTEXITCODE}: $exe $($argv -join ' ')" }
}

$Version = (Get-Content (Join-Path $Root "config\version.json") -Raw | ConvertFrom-Json).app_info.version
Write-Host ""
Write-Host "DDS Studio v$Version — сборка Windows (egui)" -ForegroundColor White
Write-Host ""
$t0 = Get-Date

# 1. окружение
Need "cargo" "установите Rust: https://rustup.rs (toolchain stable-msvc)"
Ok "инструменты найдены"

# 2. версии
if (-not $SkipVersionCheck) {
    $py = if (Get-Command python -ErrorAction SilentlyContinue) { "python" } elseif (Get-Command py -ErrorAction SilentlyContinue) { "py" } else { $null }
    if ($py) {
        Info "проверка версий (tools\update_version.py status)"
        & $py tools\update_version.py status | Out-Host
        if ($LASTEXITCODE -ne 0) { Fail "версии расходятся — выполните: $py tools\update_version.py sync" }
        Ok "версия v$Version синхронизирована"
    } else {
        Write-Host "  (Python не найден — проверка версий пропущена)" -ForegroundColor Yellow
    }
}

# 3. тесты
Info "тесты dds-core и dds-egui"
Run "cargo" @("test", "-q", "-p", "dds-core", "-p", "dds-egui")

# 4. сборка
Info "сборка приложения (cargo build --release -p dds-egui)"
Run "cargo" @("build", "--release", "-p", "dds-egui")

$out = Join-Path $Root "release\v$Version"
New-Item -ItemType Directory -Force -Path $out | Out-Null
$produced = @()

$stage = Join-Path $Root "target\win-bundle\DDS Studio"
Remove-Item (Split-Path $stage) -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item (Join-Path $Root "target\release\dds-egui.exe") (Join-Path $stage "DDS Studio.exe") -Force
Copy-Item (Join-Path $Root "LICENSE") $stage -Force
$zip = Join-Path $out "$Slug-v$Version-windows-x64.zip"
Remove-Item $zip -Force -ErrorAction SilentlyContinue
Compress-Archive -Path $stage -DestinationPath $zip
$produced += $zip

if ($WithServer) {
    Info "сборка dds-server"
    Run "cargo" @("build", "--release", "-p", "dds-server")
    $dst = Join-Path $out "dds-server-v$Version-x64.exe"
    Copy-Item (Join-Path $Root "target\release\dds-server.exe") $dst -Force
    $produced += $dst
}

# 5. контрольные суммы
$lines = foreach ($p in $produced) {
    $h = FileSha256 $p
    "$h  $(Split-Path $p -Leaf)"
}
Set-Content -Path (Join-Path $out "checksum.txt") -Value $lines -Encoding UTF8

$exe = Join-Path $stage "DDS Studio.exe"
$sum = Join-Path $out "checksum.txt"

Write-Host ""
Ok ("готово за {0:N0} с" -f ((Get-Date) - $t0).TotalSeconds)
Write-Host ""
Write-Host "  ЗАПУСТИТЬ СЕЙЧАС (распаковывать не нужно):" -ForegroundColor White
Write-Host ("    {0}   ({1:N1} МБ)" -f $exe, ((Get-Item $exe).Length / 1MB)) -ForegroundColor Green
Write-Host ("    & `"{0}`"" -f $exe) -ForegroundColor DarkGray
Write-Host ""
Write-Host "  ДИСТРИБУТИВ (что отдавать другим):" -ForegroundColor White
foreach ($p in $produced) {
    Write-Host ("    {0}   ({1:N1} МБ)" -f $p, ((Get-Item $p).Length / 1MB)) -ForegroundColor Green
}
Write-Host ("    {0}" -f $sum) -ForegroundColor DarkGray
if (-not $WithServer) {
    Write-Host "    (dds-server.exe не собирался — добавьте ключ -WithServer)" -ForegroundColor DarkGray
}
Write-Host ""
Write-Host "  Открыть папку с дистрибутивом:" -ForegroundColor White
Write-Host ("    explorer `"{0}`"" -f $out) -ForegroundColor DarkGray
Write-Host "    (или запускайте сборку с ключом -Open)" -ForegroundColor DarkGray
Write-Host ""
Write-Host "Установщика нет и не требуется: приложение — один самодостаточный .exe, архив нужен только для передачи." -ForegroundColor DarkGray
Write-Host "Программа не подписана — SmartScreen может показать предупреждение: «Подробнее» -> «Выполнить в любом случае»." -ForegroundColor Yellow
if ($Open) { Invoke-Item $out }
