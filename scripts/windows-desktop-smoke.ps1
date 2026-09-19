param(
  [switch]$Attended,
  [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,
  [string]$EvidenceDir = $null,
  [int]$FixturePort = 8481,
  [int]$SmokeTimeoutMs = 30000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (![System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows)) {
  throw "This smoke must run on Windows"
}

if (!$Attended) {
  throw "This smoke opens a native window and must be run from an attended Windows desktop session. Re-run with -Attended."
}
if ($FixturePort -le 0 -or $FixturePort -gt 65535) {
  throw "FixturePort must be in the range 1..65535"
}
if ($SmokeTimeoutMs -le 0) {
  throw "SmokeTimeoutMs must be greater than 0"
}

$resolvedRepoRoot = Resolve-Path $RepoRoot
$RepoRoot = $resolvedRepoRoot.ProviderPath
if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
  $RepoRoot = $resolvedRepoRoot.Path
}
$RepoRoot = [System.IO.Path]::GetFullPath($RepoRoot)
$env:SERVOKIT_REPO_ROOT = $RepoRoot
if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
  $EvidenceDir = Join-Path ([System.IO.Path]::GetTempPath()) "servokit-windows-desktop-smoke"
}
$EvidenceDir = [System.IO.Path]::GetFullPath($EvidenceDir)

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
Set-Location $RepoRoot

if (!(Test-Path (Join-Path $RepoRoot "examples/desktop-winit/Cargo.toml"))) {
  throw "Run from a ServoKit checkout or pass -RepoRoot"
}
if (!(Test-Path (Join-Path $RepoRoot "examples/fixtures/smoke/index.html"))) {
  throw "Fixture page not found under examples/fixtures"
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
    $RepoRoot.StartsWith("\", [System.StringComparison]::Ordinal)) {
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
if ($RepoRoot.StartsWith("\", [System.StringComparison]::Ordinal)) {
  $gitSafeDirectory = "%(prefix)/" + ($RepoRoot -replace "\\", "/")
  $gitConfigArgs = @(
    "-c", "core.fsmonitor=false",
    "-c", "core.autocrlf=false",
    "-c", "safe.directory=$gitSafeDirectory"
  )
}

function Invoke-RepoGit {
  param(
    [Parameter(Mandatory = $true)][string[]]$Arguments,
    [Parameter(Mandatory = $true)][string]$Action
  )

  $output = & git @gitConfigArgs -C $RepoRoot @Arguments 2>&1
  if ($LASTEXITCODE -ne 0) {
    $output | ForEach-Object { Write-Host $_ }
    throw "Failed to $Action"
  }
  return $output
}

function Resolve-Python {
  $python = Get-Command python -ErrorAction SilentlyContinue
  if ($null -ne $python) {
    return @{
      File = $python.Source
      Args = @()
    }
  }

  $py = Get-Command py -ErrorAction SilentlyContinue
  if ($null -ne $py) {
    return @{
      File = $py.Source
      Args = @("-3")
    }
  }

  throw "Neither python nor py was found for the fixture server"
}

function Join-ProcessArguments {
  param(
    [Parameter(Mandatory = $true)][string[]]$Arguments
  )

  ($Arguments | ForEach-Object {
    if ($_ -match '[\s"]') {
      '"' + ($_ -replace '"', '\"') + '"'
    } else {
      $_
    }
  }) -join " "
}

$vsdev = Resolve-VsDevCmd
$env:SERVOKIT_VSDEVCMD = $vsdev
$fixtureUrl = "http://127.0.0.1:$FixturePort/smoke/index.html"
$fixtureOutLog = Join-Path $EvidenceDir "fixture-server.stdout.txt"
$fixtureErrLog = Join-Path $EvidenceDir "fixture-server.stderr.txt"
$smokeLog = Join-Path $EvidenceDir "desktop-winit-smoke.txt"
$smokeCommand = "cargo run --release --locked --manifest-path examples/desktop-winit/Cargo.toml -- --smoke --smoke-timeout-ms $SmokeTimeoutMs $fixtureUrl"

@(
  "repoRoot=$RepoRoot"
  "servokitRepoRoot=$env:SERVOKIT_REPO_ROOT"
  "evidenceDir=$EvidenceDir"
  "fixtureUrl=$fixtureUrl"
  "smokeTimeoutMs=$SmokeTimeoutMs"
  "cargoBuildTarget=$env:CARGO_BUILD_TARGET"
  "cargoTermColor=$env:CARGO_TERM_COLOR"
  "rustBacktrace=$env:RUST_BACKTRACE"
  "cargoTargetDir=$cargoTargetDir"
  "userInteractive=$([Environment]::UserInteractive)"
  "vsDevCmd=$vsdev"
  "gitSafeDirectory=$gitSafeDirectory"
) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "environment.txt")

Invoke-RepoGit -Arguments @("rev-parse", "HEAD") -Action "record the Git commit" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "commit.txt")
Invoke-RepoGit -Arguments @("status", "--short", "--branch", "--ignore-submodules=dirty") -Action "record Git status" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "git-status.txt")
Invoke-RepoGit -Arguments @("diff", "--ignore-submodules=dirty", "HEAD", "--stat") -Action "record Git diff statistics" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "git-diff-stat.txt")
Invoke-RepoGit -Arguments @("diff", "--ignore-submodules=dirty", "HEAD", "--name-status") -Action "record changed paths" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "git-diff-name-status.txt")
Invoke-RepoGit -Arguments @("submodule", "status", "--recursive") -Action "record submodule status" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "submodules.txt")
systeminfo | Out-File -Encoding utf8 (Join-Path $EvidenceDir "systeminfo.txt")
"$($PSVersionTable.PSEdition) $($PSVersionTable.PSVersion)" | Out-File -Encoding utf8 (Join-Path $EvidenceDir "powershell.txt")
$python = Resolve-Python
$fixtureDirectory = Join-Path $RepoRoot "examples/fixtures"
$fixtureArgs = @($python["Args"]) + @(
  "-m",
  "http.server",
  "$FixturePort",
  "--directory",
  $fixtureDirectory
)
$fixtureArgumentLine = Join-ProcessArguments -Arguments $fixtureArgs
@(
  "fixture=$($python["File"]) $fixtureArgumentLine"
  "smoke=$smokeCommand"
) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "commands.txt")
$fixtureServer = $null

try {
  $fixtureServer = Start-Process `
    -FilePath $python["File"] `
    -ArgumentList $fixtureArgumentLine `
    -WorkingDirectory $RepoRoot `
    -RedirectStandardOutput $fixtureOutLog `
    -RedirectStandardError $fixtureErrLog `
    -PassThru `
    -WindowStyle Hidden

  Start-Sleep -Seconds 2
  if ($fixtureServer.HasExited) {
    Get-Content $fixtureOutLog -ErrorAction SilentlyContinue
    Get-Content $fixtureErrLog -ErrorAction SilentlyContinue
    throw "Fixture server exited before smoke started"
  }

  Invoke-LoggedDevCommand `
    -VsDevCmd $vsdev `
    -CommandLine $smokeCommand `
    -LogPath $smokeLog `
    -Action "run attended desktop-winit Windows smoke"

  $smoke = Get-Content $smokeLog -Raw
  $required = @(
    "smoke result=pass",
    "smoke action=input-probe",
    "input_probe=complete",
    "smoke action=reattach-cycle",
    "surface_attach_count=2",
    "surface_detach_count=2",
    "reattach=complete"
  )
  foreach ($needle in $required) {
    if (!$smoke.Contains($needle)) {
      throw "Smoke output did not contain required evidence: $needle"
    }
  }

  @(
    "status=passed"
    "repoRoot=$RepoRoot"
    "evidenceDir=$EvidenceDir"
    "fixtureUrl=$fixtureUrl"
    "smokeTimeoutMs=$SmokeTimeoutMs"
    "cargoBuildTarget=$env:CARGO_BUILD_TARGET"
    "vsDevCmd=$vsdev"
  ) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "summary.txt")
  Get-Content (Join-Path $EvidenceDir "summary.txt")
} finally {
  if ($null -ne $fixtureServer -and !$fixtureServer.HasExited) {
    Stop-Process -Id $fixtureServer.Id -Force
  }
}
