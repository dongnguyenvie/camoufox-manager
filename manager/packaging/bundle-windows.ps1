# Build a release and package it as dist\camoufox-manager-<version>-windows-x64.zip.
# Plain zip, no installer, no code signing. Mirror of bundle-macos.sh.
# Data is NOT inside the zip: the app uses %APPDATA%\camoufox-manager (camoufox-manager.db, .env, camoufox\...).
# manager\ is the Cargo workspace; the binary is in apps\camoufox-manager. The python helper is embedded
# in the binary and written to the data dir at runtime, so it is not shipped as a file.
#
# Requirements: Windows 10/11 x64, Rust (MSVC toolchain), PowerShell 5.1+ or pwsh.
# Run from anywhere:  powershell -NoProfile -ExecutionPolicy Bypass -File manager\packaging\bundle-windows.ps1
# Optional: $env:UV_BIN = "C:\path\to\uv.exe" to skip the download (local builds/tests).
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Root = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $Root

# uv is bundled so the in-app Camoufox install works with nothing preinstalled
$UvVersion = '0.12.17'
$UvZipName = 'uv-x86_64-pc-windows-msvc.zip'
$UvSha256 = 'a252121d5b59398fcb137c6ea448176459a44010f33f67e0072305a637119ca7'

# workspace-wide version ([workspace.package] in Cargo.toml)
$VersionMatch = Select-String -Path 'Cargo.toml' -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if (-not $VersionMatch) { throw 'version not found in Cargo.toml' }
$Version = $VersionMatch.Matches[0].Groups[1].Value

cargo build --release -p camoufox-manager
if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }

$Name = "camoufox-manager-$Version-windows-x64"
$Dist = Join-Path $Root 'dist'
$Stage = Join-Path $Dist $Name
$Zip = Join-Path $Dist "$Name.zip"

if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
if (Test-Path $Zip) { Remove-Item -Force $Zip }
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Stage 'licenses') | Out-Null

Copy-Item -Force (Join-Path $Root 'target\release\camoufox-manager.exe') (Join-Path $Stage 'camoufox-manager.exe')

# bundle uv: UV_BIN (local builds/tests, no download) or the pinned official release
if ($env:UV_BIN) {
    Copy-Item -Force $env:UV_BIN (Join-Path $Stage 'uv.exe')
} else {
    $Cache = Join-Path $Root 'packaging\.cache'
    New-Item -ItemType Directory -Force -Path $Cache | Out-Null
    $UvZip = Join-Path $Cache "uv-$UvVersion-$UvZipName"
    if (-not (Test-Path $UvZip)) {
        Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/astral-sh/uv/releases/download/$UvVersion/$UvZipName" -OutFile $UvZip
    }
    $Actual = (Get-FileHash -Algorithm SHA256 -Path $UvZip).Hash.ToLowerInvariant()
    if ($Actual -ne $UvSha256) {
        Remove-Item -Force $UvZip
        throw "uv sha256 mismatch: expected $UvSha256, got $Actual"
    }
    $UvTmp = Join-Path ([System.IO.Path]::GetTempPath()) ("uv-" + [System.Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $UvTmp | Out-Null
    try {
        Expand-Archive -Path $UvZip -DestinationPath $UvTmp -Force
        Copy-Item -Force (Join-Path $UvTmp 'uv.exe') (Join-Path $Stage 'uv.exe')
    } finally {
        Remove-Item -Recurse -Force $UvTmp
    }
}

# third-party licenses (uv is distributed under its MIT option)
Copy-Item -Force (Join-Path $Root 'packaging\licenses\uv-LICENSE-MIT') (Join-Path $Stage 'licenses\')

# Plain zip with the files at its top level, so unzipping gives a folder that runs in place.
Compress-Archive -Path (Join-Path $Stage '*') -DestinationPath $Zip -Force
Remove-Item -Recurse -Force $Stage

$Hash = (Get-FileHash -Algorithm SHA256 -Path $Zip).Hash.ToLowerInvariant()
Write-Output "Done: $Zip"
Write-Output "sha256: $Hash"
