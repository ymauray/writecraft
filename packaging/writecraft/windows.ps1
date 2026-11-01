<#
.SYNOPSIS
  WriteCraft: build and package WriteCraft for Windows.

.DESCRIPTION
  Produces, in $env:DIST (default: dist/release):
    writecraft-<version>-windows-<arch>.msi            installer (WiX v5), per user by default (no
                                                       administrator), into %LOCALAPPDATA%\Programs\WriteCraft
    writecraft-<version>-windows-<arch>-portable.zip   writecraft.exe and the notices

  WriteCraft's own installer (writecraft.wxs: name, UpgradeCode, folder, registry, ProgIds) and
  executable name, so it installs beside WordCraft. Adapted from WordCraft's
  packaging/windows/package.ps1, which stays as upstream has it. Not signed: Windows SmartScreen
  asks to confirm the first launch ("More info" › "Run anyway").

  Env: WRITECRAFT_VERSION (default 0.0.0-dev; the release workflow sets it from the tag),
  CRAFT_I18N_DIR (craft-i18n checkout; CRAFT_I18N_REQUIRED=1 makes it mandatory).

  Needs: Rust (MSVC toolchain), the Windows SDK (rc.exe) and WiX v5 with its UI and Util extensions:
    dotnet tool install --global wix --version 5.0.2
    wix extension add -g WixToolset.UI.wixext/5.0.2
    wix extension add -g WixToolset.Util.wixext/5.0.2

.EXAMPLE
  pwsh packaging/writecraft/windows.ps1 -Arch x64
#>
param(
  [ValidateSet('x64', 'arm64')] [string] $Arch = 'x64',
  [switch] $SkipBuild
)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

function Invoke-Native([string] $What, [scriptblock] $Block) {
  Write-Output "==> $What"
  & $Block
  if ($LASTEXITCODE -ne 0) { throw "$What failed with exit code $LASTEXITCODE" }
}

$Version = if ($env:WRITECRAFT_VERSION) { $env:WRITECRAFT_VERSION } else { '0.0.0-dev' }
$env:WRITECRAFT_VERSION = $Version
# MSI ProductVersion is numeric (major.minor.build); pre-release tags are dropped there.
$MsiVersion = ($Version -split '-')[0]

$Target = switch ($Arch) { 'x64' { 'x86_64-pc-windows-msvc' } 'arm64' { 'aarch64-pc-windows-msvc' } }
$Dist = if ($env:DIST) { $env:DIST } else { Join-Path $Root 'dist\release' }
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
New-Item -ItemType Directory -Force -Path $Dist | Out-Null

if (-not $env:WORDCRAFT_BUILD_SHA) { $env:WORDCRAFT_BUILD_SHA = (git -C $Root rev-parse HEAD 2>$null) }
if (-not $env:WORDCRAFT_BUILD_DATE) { $env:WORDCRAFT_BUILD_DATE = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd') }

Write-Output "WriteCraft $Version for Windows $Arch ($Target)"

if (-not $SkipBuild) {
  # Static CRT: no VC++ redistributable needed. Scoped to the target so host build scripts and
  # proc-macros are unaffected.
  $flagVar = 'CARGO_TARGET_' + ($Target.ToUpper() -replace '-', '_') + '_RUSTFLAGS'
  [Environment]::SetEnvironmentVariable($flagVar, '-C target-feature=+crt-static')
  # Fail the build (rather than warn) if the icon/VERSIONINFO can't be embedded.
  $env:WORDCRAFT_REQUIRE_WINRES = '1'
  Invoke-Native "cargo build ($Target)" { cargo build --release --locked -p wordcraft --target $Target }
}

$Bin = Join-Path $TargetDir "$Target\release"
$Stage = Join-Path $TargetDir "writecraft-windows\$Arch"
Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item (Join-Path $Bin 'wordcraft.exe') (Join-Path $Stage 'writecraft.exe')

$AppNotices = Join-Path $Stage 'app-notices'
New-Item -ItemType Directory -Force -Path $AppNotices | Out-Null
foreach ($f in 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'ATTRIBUTION.md') {
  $p = Join-Path $Root $f
  if (Test-Path $p) { Copy-Item -LiteralPath $p -Destination $AppNotices }
}

# ---- MSI ---------------------------------------------------------------------------------------
# The installer's welcome page shows the license, built from LICENSE-MIT and LICENSE-APACHE.
function ConvertTo-RtfText([string] $Text) {
  $sb = [System.Text.StringBuilder]::new()
  foreach ($ch in $Text.ToCharArray()) {
    $c = [int] $ch
    if ($c -eq 0x5C -or $c -eq 0x7B -or $c -eq 0x7D) { [void] $sb.Append('\').Append($ch) }
    elseif ($c -gt 127) { [void] $sb.Append("\u$(if ($c -gt 32767) { $c - 65536 } else { $c })?") }
    else { [void] $sb.Append($ch) }
  }
  return $sb.ToString()
}
$paragraphs = [System.Collections.Generic.List[string]]::new()
$paragraphs.Add('\b WriteCraft\b0')
$paragraphs.Add((ConvertTo-RtfText 'WriteCraft, based on WordCraft, is dual-licensed under the MIT License or the Apache License, Version 2.0, at your option. Both license texts follow. Third-party material keeps its own open licenses, listed in NOTICE and ATTRIBUTION.md at https://github.com/ymauray/writecraft.'))
foreach ($file in 'LICENSE-MIT', 'LICENSE-APACHE') {
  $title = $true
  foreach ($para in ((Get-Content -Raw (Join-Path $Root $file)) -split '\r?\n\s*\r?\n')) {
    $joined = (($para -split '\r?\n') | ForEach-Object { $_.Trim() } | Where-Object { $_ }) -join ' '
    if (-not $joined) { continue }
    $paragraphs.Add($(if ($title) { "\b $(ConvertTo-RtfText $joined)\b0" } else { ConvertTo-RtfText $joined }))
    $title = $false
  }
}
$LicenseRtf = Join-Path $Stage 'license.rtf'
$rtf = '{\rtf1\ansi\ansicpg1252\deff0{\fonttbl{\f0\fswiss\fcharset0 Tahoma;}}\viewkind4\uc1\pard\sa120\f0\fs16 ' +
  ($paragraphs -join "\par`r`n") + "\par`r`n}`r`n"
[System.IO.File]::WriteAllText($LicenseRtf, $rtf, [System.Text.Encoding]::ASCII)

$Msi = Join-Path $Dist "writecraft-$Version-windows-$Arch.msi"
Invoke-Native "wix build" {
  wix build (Join-Path $PSScriptRoot 'writecraft.wxs') -arch $Arch `
    -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext `
    -d "Version=$MsiVersion" -d "BinDir=$Stage" -d "IconPath=$(Join-Path $Root 'assets\writecraft-icon\writecraft.ico')" `
    -d "LicenseRtf=$LicenseRtf" -d "AppNoticesDir=$AppNotices" `
    -o $Msi
}
Remove-Item -Force -ErrorAction SilentlyContinue ([IO.Path]::ChangeExtension($Msi, '.wixpdb'))

# ---- portable zip ------------------------------------------------------------------------------
$Portable = Join-Path $TargetDir "writecraft-windows\writecraft-$Version-windows-$Arch-portable"
Remove-Item -Recurse -Force $Portable -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Portable | Out-Null
Copy-Item (Join-Path $Stage 'writecraft.exe') $Portable
Copy-Item (Join-Path $AppNotices '*') $Portable
$Zip = Join-Path $Dist "writecraft-$Version-windows-$Arch-portable.zip"
Remove-Item -Force $Zip -ErrorAction SilentlyContinue
Compress-Archive -Path $Portable -DestinationPath $Zip

Get-Item $Msi, $Zip | Format-Table Name, Length
