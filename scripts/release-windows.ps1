# Release build for Windows x64 (MSVC): zip via manager\packaging\bundle-windows.ps1, SHA256SUMS.txt
# in release\<version>\. See scripts\README.md.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts\release-windows.ps1 [-AllowDirty]
# Optional signing: $env:CM_SIGN_CERT (path to .pfx) and $env:CM_SIGN_PASSWORD (needs signtool in PATH).
param([switch]$AllowDirty)
$ErrorActionPreference = 'Stop'

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $Root

$VersionMatch = Select-String -Path 'manager\Cargo.toml' -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if (-not $VersionMatch) { throw 'version not found in manager\Cargo.toml' }
$Version = $VersionMatch.Matches[0].Groups[1].Value

$Dirty = git status --porcelain
if ($LASTEXITCODE -ne 0) { throw 'git status failed' }
if ($Dirty -and -not $AllowDirty) { throw 'git tree is dirty; commit or stash, or pass -AllowDirty' }

# the bundle script runs a plain release build, so the host toolchain must be x64 MSVC
$HostTriple = (rustc -vV | Select-String '^host:\s*(\S+)').Matches[0].Groups[1].Value
if ($HostTriple -ne 'x86_64-pc-windows-msvc') { throw "host toolchain is $HostTriple; need x86_64-pc-windows-msvc" }

& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $Root 'manager\packaging\bundle-windows.ps1')
if ($LASTEXITCODE -ne 0) { throw "bundle-windows.ps1 failed with exit code $LASTEXITCODE" }

$Built = Join-Path $Root "manager\dist\camoufox-manager-$Version-windows-x64.zip"
$Out = Join-Path $Root "release\$Version"
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Name = "Camoufox-Manager-$Version-windows-x64.zip"
$Zip = Join-Path $Out $Name
if (Test-Path $Zip) { Remove-Item -Force $Zip }

if ($env:CM_SIGN_CERT -and $env:CM_SIGN_PASSWORD) {
    # sign the exe inside the zip: unpack, sign, repack
    $Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("cm-sign-" + [System.Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $Tmp | Out-Null
    try {
        Expand-Archive -Path $Built -DestinationPath $Tmp -Force
        & signtool sign /fd SHA256 /f $env:CM_SIGN_CERT /p $env:CM_SIGN_PASSWORD /tr http://timestamp.digicert.com /td SHA256 (Join-Path $Tmp 'camoufox-manager.exe')
        if ($LASTEXITCODE -ne 0) { throw "signtool failed with exit code $LASTEXITCODE" }
        Compress-Archive -Path (Join-Path $Tmp '*') -DestinationPath $Zip -Force
    } finally {
        Remove-Item -Recurse -Force $Tmp
    }
} else {
    Write-Output 'NOTE: CM_SIGN_CERT / CM_SIGN_PASSWORD not set; skipping Authenticode signing (SmartScreen will warn).'
    Copy-Item -Force $Built $Zip
}

# SHA256SUMS.txt: keep other platforms' lines, replace ours (format "<hash>  <file>", like shasum)
$Hash = (Get-FileHash -Algorithm SHA256 -Path $Zip).Hash.ToLowerInvariant()
$Sums = Join-Path $Out 'SHA256SUMS.txt'
$Lines = @()
if (Test-Path $Sums) { $Lines = @(Get-Content $Sums | Where-Object { $_ -and ($_ -notlike "*  $Name") }) }
$Lines += "$Hash  $Name"
[System.IO.File]::WriteAllText($Sums, (($Lines -join "`n") + "`n"))

Write-Output "Done: $Zip"
Write-Output "sha256: $Hash"
