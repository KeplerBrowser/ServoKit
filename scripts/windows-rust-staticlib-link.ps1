param(
  [Parameter(Mandatory = $true)][string]$ManifestPath,
  [Parameter(Mandatory = $true)][string]$BuildLog,
  [Parameter(Mandatory = $true)][string]$ProfileDir,
  [string]$OutputProps = (Join-Path $ProfileDir "servokit_host_desktop.props"),
  [string]$EvidencePath = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (![System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows)) {
  throw "Rust static-library link metadata must be generated on Windows"
}

foreach ($requiredPath in @($ManifestPath, $BuildLog, $ProfileDir)) {
  if (!(Test-Path -LiteralPath $requiredPath)) {
    throw "Required Rust link input does not exist: $requiredPath"
  }
}

$nativeLine = Get-Content -LiteralPath $BuildLog `
  | Where-Object { $_ -like "*native-static-libs:*" } `
  | Select-Object -Last 1
if ([string]::IsNullOrWhiteSpace($nativeLine)) {
  throw "rustc did not report native-static-libs in $BuildLog"
}

$ansiEscape = [string][char]27 + '\[[0-9;?]*[ -/]*[@-~]'
$nativeText = (($nativeLine -split "native-static-libs:", 2)[1] -replace $ansiEscape, "").Trim()
$nativeLibraries = @($nativeText -split '\s+' | Where-Object { ![string]::IsNullOrWhiteSpace($_) })
if ($nativeLibraries.Count -eq 0) {
  throw "rustc reported an empty native-static-libs list"
}

$windowsImportLibrary = $nativeLibraries `
  | Where-Object { $_ -match '^windows\..+\.lib$' } `
  | Select-Object -First 1
if ([string]::IsNullOrWhiteSpace($windowsImportLibrary)) {
  throw "rustc did not report the windows-target import library"
}

$metadataStderr = Join-Path (
  [System.IO.Path]::GetTempPath()
) ("servokit-cargo-metadata-" + [System.Guid]::NewGuid().ToString("N") + ".stderr.txt")
Push-Location ([System.IO.Path]::GetTempPath())
try {
  $metadataOutput = & cargo metadata `
    --manifest-path $ManifestPath `
    --locked `
    --format-version 1 `
    2> $metadataStderr
  if ($LASTEXITCODE -ne 0) {
    Get-Content -LiteralPath $metadataStderr -ErrorAction SilentlyContinue | ForEach-Object { Write-Host $_ }
    throw "cargo metadata failed while locating $windowsImportLibrary"
  }
} finally {
  Pop-Location
}

try {
  $metadata = ($metadataOutput -join [Environment]::NewLine) | ConvertFrom-Json
} catch {
  throw "cargo metadata did not return valid JSON: $($_.Exception.Message)"
} finally {
  Remove-Item -LiteralPath $metadataStderr -Force -ErrorAction SilentlyContinue
}

$windowsImportDir = $metadata.packages `
  | Where-Object { $_.name -eq "windows_x86_64_msvc" } `
  | ForEach-Object { Join-Path (Split-Path -Parent $_.manifest_path) "lib" } `
  | Where-Object { Test-Path -LiteralPath (Join-Path $_ $windowsImportLibrary) } `
  | Select-Object -First 1
if ([string]::IsNullOrWhiteSpace($windowsImportDir)) {
  throw "Cargo metadata did not locate windows_x86_64_msvc/lib/$windowsImportLibrary"
}

$angleLibDir = Get-ChildItem -LiteralPath (Join-Path $ProfileDir "build") -Directory -Filter "mozangle-*" `
  | ForEach-Object { Join-Path $_.FullName "out" } `
  | Where-Object {
      (Test-Path -LiteralPath (Join-Path $_ "libEGL.lib")) -and
      (Test-Path -LiteralPath (Join-Path $_ "libGLESv2.lib")) -and
      (Test-Path -LiteralPath (Join-Path $_ "libEGL.dll")) -and
      (Test-Path -LiteralPath (Join-Path $_ "libGLESv2.dll"))
    } `
  | Select-Object -First 1
if ([string]::IsNullOrWhiteSpace($angleLibDir)) {
  throw "The Cargo profile did not contain the MozANGLE import and runtime libraries under $ProfileDir\build"
}

$escapeXml = {
  param([string]$Value)
  [System.Security.SecurityElement]::Escape($Value)
}
$nativeLibrariesValue = $nativeLibraries -join ";"
$propsXml = @"
<?xml version="1.0" encoding="utf-8"?>
<Project ToolsVersion="Current" xmlns="http://schemas.microsoft.com/developer/msbuild/2003">
  <PropertyGroup>
    <ServokitDesktopHostWindowsImportLibDir>$(& $escapeXml $windowsImportDir)</ServokitDesktopHostWindowsImportLibDir>
    <ServokitDesktopHostAngleLibDir>$(& $escapeXml $angleLibDir)</ServokitDesktopHostAngleLibDir>
    <ServokitDesktopHostNativeLibraries>$(& $escapeXml $nativeLibrariesValue)</ServokitDesktopHostNativeLibraries>
  </PropertyGroup>
</Project>
"@
[System.IO.File]::WriteAllText(
  [System.IO.Path]::GetFullPath($OutputProps),
  $propsXml,
  [System.Text.UTF8Encoding]::new($false)
)

$summary = @(
  "props=$([System.IO.Path]::GetFullPath($OutputProps))"
  "windowsImportLibrary=$windowsImportLibrary"
  "windowsImportDir=$windowsImportDir"
  "angleLibDir=$angleLibDir"
  "angleRuntimeLibraries=libEGL.dll;libGLESv2.dll"
  "nativeLibraries=$nativeLibrariesValue"
)
if (![string]::IsNullOrWhiteSpace($EvidencePath)) {
  $summary | Out-File -Encoding utf8 -LiteralPath $EvidencePath
}
$summary | ForEach-Object { Write-Host $_ }
