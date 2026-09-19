param(
  [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,
  [string]$EvidenceDir = $null
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (![System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows)) {
  throw "This validation slice must run on Windows"
}

$resolvedRepoRoot = Resolve-Path $RepoRoot
$RepoRoot = $resolvedRepoRoot.ProviderPath
if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
  $RepoRoot = $resolvedRepoRoot.Path
}
$RepoRoot = [System.IO.Path]::GetFullPath($RepoRoot)
$env:SERVOKIT_REPO_ROOT = $RepoRoot
if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
  $EvidenceDir = Join-Path ([System.IO.Path]::GetTempPath()) "servokit-windows-evidence"
}
$EvidenceDir = [System.IO.Path]::GetFullPath($EvidenceDir)

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
Set-Location $RepoRoot

if (!(Test-Path (Join-Path $RepoRoot "crates/Cargo.toml"))) {
  throw "Run from a ServoKit checkout or pass -RepoRoot"
}

$cargoBuildTarget = [Environment]::GetEnvironmentVariable("CARGO_BUILD_TARGET")
if ([string]::IsNullOrWhiteSpace($cargoBuildTarget)) {
  $env:CARGO_BUILD_TARGET = "x86_64-pc-windows-msvc"
  $cargoBuildTarget = $env:CARGO_BUILD_TARGET
}
if ($cargoBuildTarget -ne "x86_64-pc-windows-msvc") {
  throw "CARGO_BUILD_TARGET must be x86_64-pc-windows-msvc"
}
if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable("CARGO_TERM_COLOR"))) {
  $env:CARGO_TERM_COLOR = "always"
}
if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable("RUST_BACKTRACE"))) {
  $env:RUST_BACKTRACE = "1"
}
$cargoTargetDir = [Environment]::GetEnvironmentVariable("CARGO_TARGET_DIR")
if ([string]::IsNullOrWhiteSpace($cargoTargetDir) -and
    $RepoRoot.StartsWith("\\", [System.StringComparison]::Ordinal)) {
  $systemDrive = [Environment]::GetEnvironmentVariable("SystemDrive")
  if ([string]::IsNullOrWhiteSpace($systemDrive)) {
    throw "SystemDrive is not set"
  }
  $cargoTargetDir = Join-Path $systemDrive "servokit-target"
  $env:CARGO_TARGET_DIR = $cargoTargetDir
}

function Resolve-VsDevCmd {
  $existingVsDevCmd = [Environment]::GetEnvironmentVariable("SERVOKIT_VSDEVCMD")
  if (![string]::IsNullOrWhiteSpace($existingVsDevCmd) -and (Test-Path $existingVsDevCmd)) {
    return $existingVsDevCmd
  }

  $programFilesX86 = [Environment]::GetEnvironmentVariable("ProgramFiles(x86)")
  if ([string]::IsNullOrWhiteSpace($programFilesX86)) {
    throw "ProgramFiles(x86) is not set"
  }
  $vswhere = Join-Path $programFilesX86 "Microsoft Visual Studio\Installer\vswhere.exe"
  if (!(Test-Path $vswhere)) {
    throw "Visual Studio locator not found at $vswhere"
  }

  $installationPath = & $vswhere -latest -products "*" -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
  if ([string]::IsNullOrWhiteSpace($installationPath)) {
    throw "Visual Studio C++ x64 toolchain was not found"
  }

  $vsdev = Join-Path $installationPath "Common7\Tools\VsDevCmd.bat"
  if (!(Test-Path $vsdev)) {
    throw "Visual Studio developer command prompt not found at $vsdev"
  }
  return $vsdev
}

function Resolve-ClangBin {
  $clang = Get-Command "clang-cl.exe" -CommandType Application -ErrorAction SilentlyContinue
  if ($null -ne $clang -and
      (Test-Path $clang.Source) -and
      (Test-Path (Join-Path (Split-Path -Parent $clang.Source) "lld-link.exe"))) {
    return (Split-Path -Parent $clang.Source)
  }

  $programFiles = [Environment]::GetEnvironmentVariable("ProgramFiles")
  if (![string]::IsNullOrWhiteSpace($programFiles)) {
    $standaloneClang = Join-Path $programFiles "LLVM\bin\clang-cl.exe"
    $standaloneLinker = Join-Path $programFiles "LLVM\bin\lld-link.exe"
    if ((Test-Path $standaloneClang) -and (Test-Path $standaloneLinker)) {
      return (Split-Path -Parent $standaloneClang)
    }
  }

  throw "clang-cl.exe and lld-link.exe were not found"
}

function Resolve-Python3 {
  $candidates = @()
  $python = Get-Command "python.exe" -CommandType Application -ErrorAction SilentlyContinue
  if ($null -ne $python -and $python.Source -notlike "*\Microsoft\WindowsApps\python.exe") {
    $candidates += $python.Source
  }

  $localAppData = [Environment]::GetEnvironmentVariable("LocalAppData")
  if (![string]::IsNullOrWhiteSpace($localAppData)) {
    $pythonRoot = Join-Path $localAppData "Programs\Python"
    if (Test-Path $pythonRoot) {
      $candidates += Get-ChildItem -Path $pythonRoot -Filter "python.exe" -File -Recurse `
        | Sort-Object FullName -Descending `
        | Select-Object -ExpandProperty FullName
    }
  }

  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return $candidate
    }
  }

  throw "Python 3 interpreter was not found"
}

function Invoke-LoggedDevCommand {
  param(
    [Parameter(Mandatory = $true)][string]$VsDevCmd,
    [Parameter(Mandatory = $true)][string]$CommandLine,
    [Parameter(Mandatory = $true)][string]$LogPath,
    [Parameter(Mandatory = $true)][string]$Action
  )

  Push-Location ([System.IO.Path]::GetTempPath())
  try {
    cmd /d /s /c "(pushd `"$RepoRoot`" >nul && call `"$VsDevCmd`" -arch=x64 -host_arch=x64 >nul && $CommandLine) > `"$LogPath`" 2>&1"
    $exitCode = $LASTEXITCODE
  } finally {
    Pop-Location
  }
  if ($exitCode -ne 0) {
    Get-Content $LogPath -Tail 200
    throw "Failed to $Action"
  }
  Get-Content $LogPath -Tail 200
}

$gitConfigArgs = @()
$gitSafeDirectory = $null
if ($RepoRoot.StartsWith("\\", [System.StringComparison]::Ordinal)) {
  $gitSafeDirectory = "%(prefix)/" + ($RepoRoot -replace "\\", "/")
  $gitConfigArgs = @(
    "-c", "core.fsmonitor=false",
    "-c", "core.autocrlf=false",
    "-c", "safe.directory=$gitSafeDirectory"
  )
}

function Invoke-RepoGit {
  param(
    [string[]]$ConfigArguments = @(),
    [Parameter(Mandatory = $true)][string[]]$Arguments,
    [Parameter(Mandatory = $true)][string]$Action
  )

  $output = & git @ConfigArguments @Arguments 2>&1
  if ($LASTEXITCODE -ne 0) {
    $output | ForEach-Object { Write-Host $_ }
    throw "Failed to $Action"
  }
  return $output
}

$vsdev = Resolve-VsDevCmd
$env:SERVOKIT_VSDEVCMD = $vsdev
$clangBin = Resolve-ClangBin
if (($env:PATH -split ";") -notcontains $clangBin) {
  $env:PATH = "$clangBin;$env:PATH"
}
$env:CC = Join-Path $clangBin "clang-cl.exe"
$env:CXX = $env:CC
$python3 = Resolve-Python3
$env:PYTHON3 = $python3
$pythonBin = Split-Path -Parent $python3
if (($env:PATH -split ";") -notcontains $pythonBin) {
  $env:PATH = "$pythonBin;$env:PATH"
}

@(
  "repoRoot=$RepoRoot"
  "servokitRepoRoot=$env:SERVOKIT_REPO_ROOT"
  "evidenceDir=$EvidenceDir"
  "cargoBuildTarget=$env:CARGO_BUILD_TARGET"
  "cargoTermColor=$env:CARGO_TERM_COLOR"
  "rustBacktrace=$env:RUST_BACKTRACE"
  "cargoTargetDir=$cargoTargetDir"
  "vsDevCmd=$vsdev"
  "clangBin=$clangBin"
  "cc=$env:CC"
  "cxx=$env:CXX"
  "python3=$python3"
  "gitSafeDirectory=$gitSafeDirectory"
) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "environment.txt")

Invoke-RepoGit -ConfigArguments $gitConfigArgs -Arguments @("rev-parse", "HEAD") -Action "record the Git commit" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "commit.txt")
Invoke-RepoGit -ConfigArguments $gitConfigArgs -Arguments @("status", "--short", "--branch", "--ignore-submodules=dirty") -Action "record Git status" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "git-status.txt")
Invoke-RepoGit -ConfigArguments $gitConfigArgs -Arguments @("diff", "--ignore-submodules=dirty", "HEAD", "--stat") -Action "record Git diff statistics" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "git-diff-stat.txt")
Invoke-RepoGit -ConfigArguments $gitConfigArgs -Arguments @("diff", "--ignore-submodules=dirty", "HEAD", "--name-status") -Action "record changed paths" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "git-diff-name-status.txt")
Invoke-RepoGit -ConfigArguments $gitConfigArgs -Arguments @("submodule", "status", "--recursive") -Action "record submodule status" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "submodules.txt")
systeminfo | Out-File -Encoding utf8 (Join-Path $EvidenceDir "systeminfo.txt")
"$($PSVersionTable.PSEdition) $($PSVersionTable.PSVersion)" | Out-File -Encoding utf8 (Join-Path $EvidenceDir "powershell.txt")
$sourceFiles = @(
  (Join-Path $RepoRoot "scripts/windows-host-lifecycle.ps1")
  (Join-Path $RepoRoot "scripts/windows-desktop-smoke.ps1")
  (Join-Path $RepoRoot ".github/workflows/windows-host-lifecycle.yml")
  (Join-Path $RepoRoot "crates/servokit-host-desktop/tests/windows_native_lifecycle.cpp")
  (Join-Path $RepoRoot "examples/desktop-winit/src/main.rs")
  (Join-Path $RepoRoot "examples/desktop-winit/src/smoke.rs")
)
Get-FileHash -Path $sourceFiles `
  | Format-Table -AutoSize `
  | Out-String -Width 4096 `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "source-hashes.txt")

$toolchainCommand = "rustc -vV && cargo -V && rustup show active-toolchain && rustup target list --installed && where rustc && where cargo && where rustup && clang++ --version && where clang++ && where link"
$metadataCommand = "cargo metadata --manifest-path crates/Cargo.toml --locked --format-version 1"
$cBoundaryCommand = "cargo test --manifest-path crates/Cargo.toml -p servokit-host-desktop --test c_boundary --release --locked"
$desktopWinitCommand = "cargo check --manifest-path examples/desktop-winit/Cargo.toml --locked"
@(
  "toolchain=$toolchainCommand"
  "metadata=$metadataCommand"
  "cBoundary=$cBoundaryCommand"
  "desktopWinit=$desktopWinitCommand"
) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "commands.txt")

$programFilesX86 = [Environment]::GetEnvironmentVariable("ProgramFiles(x86)")
$vswhere = Join-Path $programFilesX86 "Microsoft Visual Studio\Installer\vswhere.exe"
@(
  "vswhere=$vswhere"
  "vsDevCmd=$vsdev"
) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "visual-studio.txt")
Get-Content (Join-Path $EvidenceDir "visual-studio.txt")

$toolchainLog = Join-Path $EvidenceDir "toolchain.txt"
Invoke-LoggedDevCommand `
  -VsDevCmd $vsdev `
  -CommandLine $toolchainCommand `
  -LogPath $toolchainLog `
  -Action "record Windows toolchain evidence"

$toolchain = Get-Content $toolchainLog -Raw
if ($toolchain -notmatch "host: x86_64-pc-windows-msvc") {
  throw "Rust host target is not x86_64-pc-windows-msvc"
}

$metadataPath = Join-Path $EvidenceDir "cargo-metadata.json"
$metadataStderr = Join-Path $EvidenceDir "cargo-metadata.stderr.txt"
Push-Location ([System.IO.Path]::GetTempPath())
try {
  cmd /d /s /c "pushd `"$RepoRoot`" >nul && call `"$vsdev`" -arch=x64 -host_arch=x64 >nul && $metadataCommand > `"$metadataPath`" 2> `"$metadataStderr`""
  $metadataExitCode = $LASTEXITCODE
} finally {
  Pop-Location
}
if ($metadataExitCode -ne 0) {
  Get-Content $metadataStderr
  throw "Failed to record Rust dependency metadata"
}
$metadata = Get-Content $metadataPath -Raw | ConvertFrom-Json
$servo = $metadata.packages | Where-Object { $_.name -eq "servo" } | Select-Object -First 1
if ($null -eq $servo) {
  throw "Cargo metadata did not include the servo package"
}
"servo $($servo.version) source=$($servo.source)" | Out-File -Encoding utf8 (Join-Path $EvidenceDir "servo-version.txt")
Get-Content (Join-Path $EvidenceDir "servo-version.txt")

Invoke-LoggedDevCommand `
  -VsDevCmd $vsdev `
  -CommandLine $cBoundaryCommand `
  -LogPath (Join-Path $EvidenceDir "servokit-host-desktop-c-boundary.txt") `
  -Action "run desktop C boundary and Win32 lifecycle harness"

Invoke-LoggedDevCommand `
  -VsDevCmd $vsdev `
  -CommandLine $desktopWinitCommand `
  -LogPath (Join-Path $EvidenceDir "desktop-winit-check.txt") `
  -Action "check desktop winit proof app"

@(
  "status=passed"
  "repoRoot=$RepoRoot"
  "evidenceDir=$EvidenceDir"
  "cargoBuildTarget=$env:CARGO_BUILD_TARGET"
  "cargoTermColor=$env:CARGO_TERM_COLOR"
  "rustBacktrace=$env:RUST_BACKTRACE"
  "vsDevCmd=$vsdev"
) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "summary.txt")
Get-Content (Join-Path $EvidenceDir "summary.txt")
