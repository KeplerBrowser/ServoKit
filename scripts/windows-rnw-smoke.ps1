param(
  [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,
  [string]$PackageRoot = $null,
  [string]$AppRoot = $null,
  [string]$EvidenceDir = $null,
  [ValidateSet("Debug", "Release")][string]$Configuration = "Debug",
  [int]$FixturePort = 8481,
  [int]$MetroPort = 8081,
  [switch]$Attended
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (![System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows)) {
  throw "This smoke must run on Windows"
}

if (!$Attended) {
  throw "RNW runtime smoke launches an app and requires human verification. Re-run with -Attended."
}
if ($FixturePort -lt 1 -or $FixturePort -gt 65535) {
  throw "FixturePort must be in the range 1..65535"
}
if ($MetroPort -lt 1 -or $MetroPort -gt 65535) {
  throw "MetroPort must be in the range 1..65535"
}

$resolvedRepoRoot = Resolve-Path $RepoRoot
$RepoRoot = $resolvedRepoRoot.ProviderPath
if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
  $RepoRoot = $resolvedRepoRoot.Path
}
$RepoRoot = [System.IO.Path]::GetFullPath($RepoRoot)
$env:SERVOKIT_REPO_ROOT = $RepoRoot
if ([string]::IsNullOrWhiteSpace($PackageRoot)) {
  $PackageRoot = Join-Path $RepoRoot "packages/react-native-servokit"
}
$resolvedPackageRoot = Resolve-Path $PackageRoot
$PackageRoot = $resolvedPackageRoot.ProviderPath
if ([string]::IsNullOrWhiteSpace($PackageRoot)) {
  $PackageRoot = $resolvedPackageRoot.Path
}
$PackageRoot = [System.IO.Path]::GetFullPath($PackageRoot)
if ([string]::IsNullOrWhiteSpace($AppRoot)) {
  $AppRoot = Join-Path $PackageRoot "example"
}
$resolvedAppRoot = Resolve-Path $AppRoot
$AppRoot = $resolvedAppRoot.ProviderPath
if ([string]::IsNullOrWhiteSpace($AppRoot)) {
  $AppRoot = $resolvedAppRoot.Path
}
$AppRoot = [System.IO.Path]::GetFullPath($AppRoot)
if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
  $EvidenceDir = Join-Path ([System.IO.Path]::GetTempPath()) "servokit-windows-rnw-smoke"
}
$EvidenceDir = [System.IO.Path]::GetFullPath($EvidenceDir)
$smokeUrl = "http://127.0.0.1:$FixturePort/smoke/index.html"

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
Set-Location $RepoRoot

if (!(Test-Path (Join-Path $RepoRoot "crates/Cargo.toml"))) {
  throw "Run from a ServoKit checkout or pass -RepoRoot"
}
if (!(Test-Path (Join-Path $PackageRoot "windows/ServoKit.sln"))) {
  throw "PackageRoot must contain windows/ServoKit.sln"
}
if (!(Test-Path (Join-Path $AppRoot "package.json"))) {
  throw "AppRoot must contain a React Native package.json"
}
if (!(Test-Path (Join-Path $AppRoot "windows"))) {
  throw "AppRoot must contain a React Native Windows app project. Run init-windows for the app or pass -AppRoot to an existing RNW app."
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

function Invoke-LoggedDevCommand {
  param(
    [Parameter(Mandatory = $true)][string]$VsDevCmd,
    [Parameter(Mandatory = $true)][string]$WorkingDirectory,
    [Parameter(Mandatory = $true)][string]$CommandLine,
    [Parameter(Mandatory = $true)][string]$LogPath,
    [Parameter(Mandatory = $true)][string]$Action
  )

  Push-Location ([System.IO.Path]::GetTempPath())
  try {
    cmd /d /s /c "(pushd `"$WorkingDirectory`" >nul && call `"$VsDevCmd`" -arch=x64 -host_arch=x64 >nul && $CommandLine) > `"$LogPath`" 2>&1"
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

function Resolve-AppSmokeSource {
  param(
    [Parameter(Mandatory = $true)][string]$AppRoot,
    [Parameter(Mandatory = $true)][string]$SmokeUrl
  )

  $requiredMarkers = @(
    "ServoView",
    $SmokeUrl,
    "onUrlChanged",
    "onLoadStatusChanged",
    "onHistoryChanged",
    "onFocusChanged",
    "onShouldStartLoadWithRequest",
    "onJavaScriptDialog",
    "evaluateJavaScript",
    "goBack()",
    "goForward()",
    "reload()",
    "focus()",
    "blur()",
    "setServoViewGeneration",
    "action.recycle-view"
  )
  $relativeCandidates = @(
    "src/App.tsx",
    "src/App.ts",
    "src/App.jsx",
    "src/App.js",
    "App.tsx",
    "App.ts",
    "App.jsx",
    "App.js",
    "index.ts",
    "index.js"
  )

  $bestCandidate = $null
  $bestMissingMarkers = $requiredMarkers
  foreach ($relativePath in $relativeCandidates) {
    $candidate = Join-Path $AppRoot $relativePath
    if (!(Test-Path $candidate)) {
      continue
    }
    $source = Get-Content -Raw $candidate
    $missingMarkers = @($requiredMarkers | Where-Object { !$source.Contains($_) })
    if ($missingMarkers.Count -lt $bestMissingMarkers.Count) {
      $bestCandidate = $candidate
      $bestMissingMarkers = $missingMarkers
    }
    if ($missingMarkers.Count -eq 0) {
      return (Resolve-Path $candidate).Path
    }
  }

  $detail = if ($null -ne $bestCandidate) {
    "Closest candidate: $bestCandidate. Missing markers: $($bestMissingMarkers -join ', ')"
  } else {
    "No candidate app source was found."
  }
  throw "AppRoot must include a JS/TS smoke source that mounts ServoView, loads $SmokeUrl, and exposes the RNW runtime smoke controls/events. Checked: $($relativeCandidates -join ', '). $detail"
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
if ([string]::IsNullOrWhiteSpace($cargoTargetDir)) {
  $cargoTargetDir = Join-Path $RepoRoot "target"
}
$desktopHostIncludeDir = Join-Path $RepoRoot "crates/servokit-host-desktop/include"
$desktopHostProfile = if ($Configuration -eq "Debug") { "debug" } else { "release" }
$desktopHostTargetDir = Join-Path $cargoTargetDir "x86_64-pc-windows-msvc"
$desktopHostLibDir = Join-Path $desktopHostTargetDir $desktopHostProfile
$desktopHostLib = Join-Path $desktopHostLibDir "servokit_host_desktop.lib"
$fixtureRoot = Join-Path $RepoRoot "examples/fixtures"
$reactNativeBin = @(
  (Join-Path $AppRoot "node_modules/.bin/react-native.exe")
  (Join-Path $AppRoot "node_modules/.bin/react-native.cmd")
) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if ([string]::IsNullOrWhiteSpace($reactNativeBin)) {
  throw "React Native CLI binary was not found under $AppRoot\node_modules\.bin"
}
$appSmokeSource = Resolve-AppSmokeSource -AppRoot $AppRoot -SmokeUrl $smokeUrl
$rnwBuildLogDir = Join-Path $EvidenceDir "rnw-app-build"
New-Item -ItemType Directory -Force -Path $rnwBuildLogDir | Out-Null

$cargoBuildArgs = @(
  "rustc",
  "--manifest-path",
  (Join-Path $RepoRoot "crates/Cargo.toml"),
  "-p",
  "servokit-host-desktop",
  "--target",
  "x86_64-pc-windows-msvc",
  "--locked",
  "--lib"
)
if ($Configuration -eq "Release") {
  $cargoBuildArgs += "--release"
}
$cargoBuildArgs += @("--", "--print", "native-static-libs")
$cargoBuildCommand = "cargo " + (Join-ProcessArguments -Arguments $cargoBuildArgs)
$autolinkConfigCommand = "node " + (Join-ProcessArguments -Arguments @(
  (Join-Path $PackageRoot "scripts/check-windows-autolink-config.mjs"),
  "--package-root",
  $PackageRoot,
  "--example-root",
  $AppRoot,
  "--require-windows-project"
))
$runWindowsArgs = @(
  $reactNativeBin,
  "run-windows",
  "--root",
  $AppRoot,
  "--arch",
  "x64",
  "--logging",
  "--no-packager",
  "--buildLogDirectory",
  $rnwBuildLogDir,
  "--msbuildprops",
  "PlatformToolset=v143,ServokitDesktopHostIncludeDir=$desktopHostIncludeDir,ServokitDesktopHostLibDir=$desktopHostLibDir"
)
if ($Configuration -eq "Release") {
  $runWindowsArgs += "--release"
}
$runWindowsCommand = Join-ProcessArguments -Arguments $runWindowsArgs
$metroCommand = Join-ProcessArguments -Arguments @(
  $reactNativeBin,
  "start",
  "--port",
  "$MetroPort",
  "--no-interactive"
)

@(
  "repoRoot=$RepoRoot"
  "packageRoot=$PackageRoot"
  "appRoot=$AppRoot"
  "evidenceDir=$EvidenceDir"
  "configuration=$Configuration"
  "fixtureUrl=$smokeUrl"
  "appSmokeSource=$appSmokeSource"
  "metroPort=$MetroPort"
  "cargoBuildTarget=$env:CARGO_BUILD_TARGET"
  "cargoTermColor=$env:CARGO_TERM_COLOR"
  "rustBacktrace=$env:RUST_BACKTRACE"
  "desktopHostIncludeDir=$desktopHostIncludeDir"
  "desktopHostLibDir=$desktopHostLibDir"
  "desktopHostLinkProps=$(Join-Path $desktopHostLibDir 'servokit_host_desktop.props')"
  "vsDevCmd=$vsdev"
  "clangBin=$clangBin"
  "cc=$env:CC"
  "cxx=$env:CXX"
  "python3=$python3"
) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "environment.txt")

$gitConfigArgs = @()
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
Invoke-RepoGit -Arguments @("rev-parse", "HEAD") -Action "record the Git commit" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "commit.txt")
Invoke-RepoGit -Arguments @("status", "--short", "--branch", "--ignore-submodules=dirty") -Action "record Git status" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "git-status.txt")
Invoke-RepoGit -Arguments @("diff", "--ignore-submodules=dirty", "HEAD", "--stat") -Action "record Git diff statistics" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "git-diff-stat.txt")
Invoke-RepoGit -Arguments @("diff", "--ignore-submodules=dirty", "HEAD", "--name-status") -Action "record changed paths" `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "git-diff-name-status.txt")
systeminfo | Out-File -Encoding utf8 (Join-Path $EvidenceDir "systeminfo.txt")
"$($PSVersionTable.PSEdition) $($PSVersionTable.PSVersion)" | Out-File -Encoding utf8 (Join-Path $EvidenceDir "powershell.txt")

$appWindowsSourceFiles = Get-ChildItem `
  -Path (Join-Path $AppRoot "windows") `
  -Recurse `
  -Include *.sln,*.vcxproj,*.props,*.cpp,*.h,*.idl,*.def `
  -File `
  | Select-Object -ExpandProperty FullName
$sourceFileCandidates = @(
    (Join-Path $RepoRoot "scripts/windows-rnw-smoke.ps1")
    (Join-Path $RepoRoot "scripts/windows-rust-staticlib-link.ps1")
  (Join-Path $PackageRoot "package.json")
  (Join-Path $PackageRoot "react-native.config.js")
  (Join-Path $PackageRoot "src/ServoView.tsx")
  (Join-Path $PackageRoot "windows/ServoKit/ServoView.cpp")
  (Join-Path $PackageRoot "windows/ServoKit/ServoView.h")
  (Join-Path $RepoRoot "crates/servokit-host-desktop/include/servokit_desktop_private.h")
  (Join-Path $AppRoot "package.json")
  (Join-Path $AppRoot "app.json")
  (Join-Path $AppRoot "index.js")
  (Join-Path $AppRoot "metro.config.js")
  (Join-Path $AppRoot "react-native.config.js")
  $appSmokeSource
  $appWindowsSourceFiles
)
$sourceFiles = $sourceFileCandidates | Where-Object { Test-Path $_ }
Get-FileHash -Path $sourceFiles `
  | Format-Table -AutoSize `
  | Out-String -Width 4096 `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "source-hashes.txt")

@(
  "desktopHostStaticlib=$cargoBuildCommand"
  "rnwAutolinkConfig=$autolinkConfigCommand"
  "fixtureServer=python -m http.server $FixturePort --bind 127.0.0.1 --directory $fixtureRoot"
  "fixtureUrl=$smokeUrl"
  "metro=$metroCommand"
  "runWindows=$runWindowsCommand"
) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "commands.txt")

$fixtureProcess = $null
$metroProcess = $null

try {
  Invoke-LoggedDevCommand `
    -VsDevCmd $vsdev `
    -WorkingDirectory $RepoRoot `
    -CommandLine $cargoBuildCommand `
    -LogPath (Join-Path $EvidenceDir "servokit-host-desktop-build.txt") `
    -Action "build the Windows desktop host static library"

  if (!(Test-Path $desktopHostLib)) {
    throw "Desktop host static library was not produced at $desktopHostLib"
  }

  & (Join-Path $RepoRoot "scripts/windows-rust-staticlib-link.ps1") `
    -ManifestPath (Join-Path $RepoRoot "crates/Cargo.toml") `
    -BuildLog (Join-Path $EvidenceDir "servokit-host-desktop-build.txt") `
    -ProfileDir $desktopHostLibDir `
    -OutputProps (Join-Path $desktopHostLibDir "servokit_host_desktop.props") `
    -EvidencePath (Join-Path $EvidenceDir "servokit-host-desktop-link.txt")

  Invoke-LoggedDevCommand `
    -VsDevCmd $vsdev `
    -WorkingDirectory $RepoRoot `
    -CommandLine $autolinkConfigCommand `
    -LogPath (Join-Path $EvidenceDir "rnw-autolink-config.txt") `
    -Action "check React Native Windows autolinking config"

  $fixtureProcess = Start-Process `
    -FilePath $python3 `
    -ArgumentList @(
      "-m",
      "http.server",
      "$FixturePort",
      "--bind",
      "127.0.0.1",
      "--directory",
      $fixtureRoot
    ) `
    -WorkingDirectory $EvidenceDir `
    -RedirectStandardOutput (Join-Path $EvidenceDir "fixture-server.stdout.txt") `
    -RedirectStandardError (Join-Path $EvidenceDir "fixture-server.stderr.txt") `
    -WindowStyle Hidden `
    -PassThru

  if ($Configuration -eq "Debug") {
    $metroProcess = Start-Process `
      -FilePath "cmd.exe" `
      -ArgumentList @("/d", "/s", "/c", "cd /d `"$AppRoot`" && $metroCommand") `
      -WorkingDirectory $AppRoot `
      -RedirectStandardOutput (Join-Path $EvidenceDir "metro.stdout.txt") `
      -RedirectStandardError (Join-Path $EvidenceDir "metro.stderr.txt") `
      -WindowStyle Hidden `
      -PassThru
  }

  Start-Sleep -Seconds 5

  Invoke-LoggedDevCommand `
    -VsDevCmd $vsdev `
    -WorkingDirectory $AppRoot `
    -CommandLine $runWindowsCommand `
    -LogPath (Join-Path $EvidenceDir "rnw-run-windows.txt") `
    -Action "build, deploy, and launch the React Native Windows app"

  @(
    "Verify the launched RNW app renders $smokeUrl."
    "App smoke source: $appSmokeSource"
    "Exercise resize/DPI, hide/show, pointer/wheel, keyboard/text, focus, navigation commands/events, two recycle/remount cycles, teardown, and stale-handle behavior."
    "Record screenshots or notes beside this evidence directory, then type PASS at the prompt."
  ) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "attended-checklist.txt")

  $attendedResult = Read-Host "Complete the RNW smoke checklist, save screenshots/notes in $EvidenceDir, then type PASS"
  $attendedResult | Out-File -Encoding utf8 (Join-Path $EvidenceDir "attended-result.txt")
  if ($attendedResult -ne "PASS") {
    @(
      "status=attended-incomplete"
      "repoRoot=$RepoRoot"
      "packageRoot=$PackageRoot"
      "appRoot=$AppRoot"
      "evidenceDir=$EvidenceDir"
      "configuration=$Configuration"
      "desktopHostLib=$desktopHostLib"
      "fixtureUrl=$smokeUrl"
      "appSmokeSource=$appSmokeSource"
    ) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "summary.txt")
    throw "RNW smoke checklist was not confirmed with PASS"
  }

  @(
    "status=attended-pass"
    "repoRoot=$RepoRoot"
    "packageRoot=$PackageRoot"
    "appRoot=$AppRoot"
    "evidenceDir=$EvidenceDir"
    "configuration=$Configuration"
    "desktopHostLib=$desktopHostLib"
    "fixtureUrl=$smokeUrl"
    "appSmokeSource=$appSmokeSource"
  ) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "summary.txt")
  Get-Content (Join-Path $EvidenceDir "summary.txt")
} finally {
  if ($null -ne $metroProcess -and !$metroProcess.HasExited) {
    Stop-Process -Id $metroProcess.Id -Force
  }
  if ($null -ne $fixtureProcess -and !$fixtureProcess.HasExited) {
    Stop-Process -Id $fixtureProcess.Id -Force
  }
}
