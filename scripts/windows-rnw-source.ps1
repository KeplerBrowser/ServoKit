param(
  [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,
  [string]$PackageRoot = $null,
  [string]$EvidenceDir = $null,
  [ValidateSet("Debug", "Release")][string]$Configuration = "Release",
  [string]$WindowsSdkVersion = "",
  [string]$CodegenCommand = "",
  [switch]$SkipCodegen
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
if ([string]::IsNullOrWhiteSpace($PackageRoot)) {
  $PackageRoot = Join-Path $RepoRoot "packages/react-native-servokit"
}
$resolvedPackageRoot = Resolve-Path $PackageRoot
$PackageRoot = $resolvedPackageRoot.ProviderPath
if ([string]::IsNullOrWhiteSpace($PackageRoot)) {
  $PackageRoot = $resolvedPackageRoot.Path
}
$PackageRoot = [System.IO.Path]::GetFullPath($PackageRoot)
if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
  $EvidenceDir = Join-Path ([System.IO.Path]::GetTempPath()) "servokit-windows-rnw-source"
}
$EvidenceDir = [System.IO.Path]::GetFullPath($EvidenceDir)

New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
Set-Location $RepoRoot

if (!(Test-Path (Join-Path $RepoRoot "crates/Cargo.toml"))) {
  throw "Run from a ServoKit checkout or pass -RepoRoot"
}
if (!(Test-Path (Join-Path $PackageRoot "windows/ServoKit.sln"))) {
  throw "PackageRoot must contain windows/ServoKit.sln"
}
if (!(Test-Path (Join-Path $PackageRoot "src/ServoViewNativeComponent.ts"))) {
  throw "PackageRoot must contain the ServoView TypeScript codegen spec"
}
$exampleRoot = Join-Path $PackageRoot "example"
if (!(Test-Path (Join-Path $exampleRoot "package.json"))) {
  throw "PackageRoot must contain the example React Native app"
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
}
if ([string]::IsNullOrWhiteSpace($cargoTargetDir)) {
  $cargoTargetDir = Join-Path $RepoRoot "target"
}
$env:CARGO_TARGET_DIR = $cargoTargetDir

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

function Resolve-WindowsSdkVersion {
  param(
    [string]$RequestedVersion = ""
  )

  $programFilesX86 = [Environment]::GetEnvironmentVariable("ProgramFiles(x86)")
  if ([string]::IsNullOrWhiteSpace($programFilesX86)) {
    throw "ProgramFiles(x86) is not set"
  }

  $includeRoot = Join-Path $programFilesX86 "Windows Kits\10\Include"
  if (!(Test-Path $includeRoot)) {
    throw "Windows 10/11 SDK include directory was not found at $includeRoot"
  }

  $minimumVersion = [Version]"10.0.22621.0"
  $installedVersions = @(
    Get-ChildItem -LiteralPath $includeRoot -Directory | ForEach-Object {
      $parsedVersion = $null
      if ([Version]::TryParse($_.Name, [ref]$parsedVersion) -and
          $parsedVersion -ge $minimumVersion -and
          (Test-Path (Join-Path $_.FullName "um")) -and
          (Test-Path (Join-Path $_.FullName "ucrt"))) {
        [PSCustomObject]@{
          Name = $_.Name
          Version = $parsedVersion
        }
      }
    } | Sort-Object Version -Descending
  )

  if ($installedVersions.Count -eq 0) {
    throw "React Native Windows new architecture requires Windows SDK 10.0.22621.0 or newer"
  }

  if (![string]::IsNullOrWhiteSpace($RequestedVersion)) {
    $requested = $installedVersions | Where-Object Name -eq $RequestedVersion | Select-Object -First 1
    if ($null -eq $requested) {
      throw "Requested Windows SDK $RequestedVersion is not installed or is older than 10.0.22621.0"
    }
    return $requested.Name
  }

  $rnwDefault = $installedVersions | Where-Object Name -eq "10.0.22621.0" | Select-Object -First 1
  if ($null -ne $rnwDefault) {
    return $rnwDefault.Name
  }

  return $installedVersions[0].Name
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

function Resolve-RnwCodegenCommand {
  param(
    [Parameter(Mandatory = $true)][string]$PackageRoot
  )

  $resolveScript = @'
const path = require("path");
const rnwPackageJson = require.resolve("react-native-windows/package.json", { paths: [process.cwd()] });
const rnwRoot = path.dirname(rnwPackageJson);
process.stdout.write(require.resolve("@react-native-windows/codegen/bin.js", { paths: [rnwRoot] }));
'@

  $resolveScriptPath = Join-Path (
    [System.IO.Path]::GetTempPath()
  ) ("servokit-rnw-codegen-" + [System.Guid]::NewGuid().ToString("N") + ".cjs")
  [System.IO.File]::WriteAllText(
    $resolveScriptPath,
    $resolveScript,
    [System.Text.UTF8Encoding]::new($false)
  )

  Push-Location $PackageRoot
  try {
    $codegenBin = & node $resolveScriptPath
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($codegenBin)) {
      throw "Unable to resolve @react-native-windows/codegen from react-native-windows"
    }
  } finally {
    Pop-Location
    Remove-Item -LiteralPath $resolveScriptPath -Force
  }

  "node " + (Join-ProcessArguments -Arguments @(
    $codegenBin,
    "--files",
    "src/**/*Native*.[jt]s",
    "--componentsWindows",
    "--modulesWindows",
    "--libraryName",
    "ServoViewSpec",
    "--namespace",
    "ServoKitCodegen",
    "--outputDirectory",
    "windows/ServoKit/codegen",
    "--separateDataTypes",
    "--test"
  ))
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
$WindowsSdkVersion = Resolve-WindowsSdkVersion -RequestedVersion $WindowsSdkVersion
$desktopHostIncludeDir = Join-Path $RepoRoot "crates/servokit-host-desktop/include"
$desktopHostProfile = if ($Configuration -eq "Debug") { "debug" } else { "release" }
$desktopHostTargetDir = Join-Path $cargoTargetDir "x86_64-pc-windows-msvc"
$desktopHostLibDir = Join-Path $desktopHostTargetDir $desktopHostProfile
$desktopHostLib = Join-Path $desktopHostLibDir "servokit_host_desktop.lib"
$solution = Join-Path $PackageRoot "windows/ServoKit.sln"
$generatedHeader = Join-Path $PackageRoot "windows/ServoKit/codegen/react/components/ServoViewSpec/ServoView.g.h"
if ([string]::IsNullOrWhiteSpace($CodegenCommand)) {
  if ($SkipCodegen) {
    $CodegenCommand = "(skipped)"
  } else {
    $CodegenCommand = Resolve-RnwCodegenCommand -PackageRoot $PackageRoot
  }
}
$autolinkConfigCommand = "node " + (Join-ProcessArguments -Arguments @(
  (Join-Path $PackageRoot "scripts/check-windows-autolink-config.mjs"),
  "--package-root",
  $PackageRoot,
  "--example-root",
  $exampleRoot
))

$toolchainCommand = "rustc -vV && cargo -V && rustup show active-toolchain && rustup target list --installed && clang-cl --version && lld-link --version && " +
  (Join-ProcessArguments -Arguments @($python3)) +
  " --version && where rustc && where cargo && where rustup && where msbuild && where node && where clang-cl && where lld-link"
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

@(
  "repoRoot=$RepoRoot"
  "servokitRepoRoot=$env:SERVOKIT_REPO_ROOT"
  "packageRoot=$PackageRoot"
  "exampleRoot=$exampleRoot"
  "evidenceDir=$EvidenceDir"
  "configuration=$Configuration"
  "cargoBuildTarget=$env:CARGO_BUILD_TARGET"
  "cargoTermColor=$env:CARGO_TERM_COLOR"
  "rustBacktrace=$env:RUST_BACKTRACE"
  "cargoTargetDir=$cargoTargetDir"
  "desktopHostIncludeDir=$desktopHostIncludeDir"
  "desktopHostLibDir=$desktopHostLibDir"
  "vsDevCmd=$vsdev"
  "clangBin=$clangBin"
  "cc=$env:CC"
  "cxx=$env:CXX"
  "python3=$python3"
  "windowsSdkVersion=$WindowsSdkVersion"
  "gitSafeDirectory=$gitSafeDirectory"
  "skipCodegen=$SkipCodegen"
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
  (Join-Path $RepoRoot "scripts/windows-rnw-source.ps1")
  (Join-Path $RepoRoot "scripts/windows-rust-staticlib-link.ps1")
  (Join-Path $RepoRoot ".github/workflows/windows-rnw-source.yml")
  (Join-Path $PackageRoot "package.json")
  (Join-Path $PackageRoot "example/package.json")
  (Join-Path $PackageRoot "react-native.config.js")
  (Join-Path $PackageRoot "scripts/check-windows-autolink-config.mjs")
  (Join-Path $PackageRoot "src/ServoViewNativeComponent.ts")
  (Join-Path $PackageRoot "src/ServoView.tsx")
  (Join-Path $PackageRoot "windows/ServoKit.sln")
  (Join-Path $PackageRoot "windows/NuGet.Config")
  (Join-Path $PackageRoot "windows/ServoKit/ServoKit.vcxproj")
  (Join-Path $PackageRoot "windows/ServoKit/PropertySheet.props")
  (Join-Path $PackageRoot "windows/ServoKit/ServoKit.def")
  (Join-Path $PackageRoot "windows/ServoKit/pch.h")
  (Join-Path $PackageRoot "windows/ServoKit/ServoView.cpp")
  (Join-Path $PackageRoot "windows/ServoKit/ServoView.h")
  (Join-Path $PackageRoot "windows/ServoKit/ReactPackageProvider.idl")
  $generatedHeader
  (Join-Path $RepoRoot "crates/servokit-host-desktop/include/servokit_desktop_private.h")
)
Get-FileHash -Path $sourceFiles `
  | Format-Table -AutoSize `
  | Out-String -Width 4096 `
  | Out-File -Encoding utf8 (Join-Path $EvidenceDir "source-hashes.txt")

Invoke-LoggedDevCommand `
  -VsDevCmd $vsdev `
  -WorkingDirectory $RepoRoot `
  -CommandLine $toolchainCommand `
  -LogPath (Join-Path $EvidenceDir "toolchain.txt") `
  -Action "record Windows RNW toolchain evidence"

Invoke-LoggedDevCommand `
  -VsDevCmd $vsdev `
  -WorkingDirectory $RepoRoot `
  -CommandLine $cargoBuildCommand `
  -LogPath (Join-Path $EvidenceDir "servokit-host-desktop-build.txt") `
  -Action "build the Windows desktop host static library"

if (!(Test-Path $desktopHostLib)) {
  throw "Desktop host static library was not produced at $desktopHostLib"
}

$desktopHostLinkProps = Join-Path $desktopHostLibDir "servokit_host_desktop.props"
& (Join-Path $RepoRoot "scripts/windows-rust-staticlib-link.ps1") `
  -ManifestPath (Join-Path $RepoRoot "crates/Cargo.toml") `
  -BuildLog (Join-Path $EvidenceDir "servokit-host-desktop-build.txt") `
  -ProfileDir $desktopHostLibDir `
  -OutputProps $desktopHostLinkProps `
  -EvidencePath (Join-Path $EvidenceDir "servokit-host-desktop-link.txt")

$msbuildCommand = "msbuild " + (Join-ProcessArguments -Arguments @(
  $solution,
  "/restore",
  "/m",
  "/p:RestorePackagesConfig=true",
  "/p:Configuration=$Configuration",
  "/p:Platform=x64",
  "/p:WindowsTargetPlatformVersion=$WindowsSdkVersion",
  "/p:ServokitDesktopHostIncludeDir=$desktopHostIncludeDir",
  "/p:ServokitDesktopHostLibDir=$desktopHostLibDir"
))

@(
  "toolchain=$toolchainCommand"
  "desktopHostStaticlib=$cargoBuildCommand"
  "desktopHostLinkProps=$desktopHostLinkProps"
  "rnwAutolinkConfig=$autolinkConfigCommand"
  "codegen=$CodegenCommand"
  "msbuild=$msbuildCommand"
) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "commands.txt")

Invoke-LoggedDevCommand `
  -VsDevCmd $vsdev `
  -WorkingDirectory $RepoRoot `
  -CommandLine $autolinkConfigCommand `
  -LogPath (Join-Path $EvidenceDir "rnw-autolink-config.txt") `
  -Action "check React Native Windows autolinking config"

if (!$SkipCodegen) {
  Invoke-LoggedDevCommand `
    -VsDevCmd $vsdev `
    -WorkingDirectory $PackageRoot `
    -CommandLine $CodegenCommand `
    -LogPath (Join-Path $EvidenceDir "rnw-codegen.txt") `
    -Action "check React Native Windows codegen"

  if (!(Test-Path $generatedHeader)) {
    throw "React Native Windows codegen did not produce $generatedHeader"
  }
}

Invoke-LoggedDevCommand `
  -VsDevCmd $vsdev `
  -WorkingDirectory $RepoRoot `
  -CommandLine $msbuildCommand `
  -LogPath (Join-Path $EvidenceDir "rnw-msbuild.txt") `
  -Action "build the React Native Windows ServoKit source project"

@(
  "status=passed"
  "repoRoot=$RepoRoot"
  "packageRoot=$PackageRoot"
  "evidenceDir=$EvidenceDir"
  "configuration=$Configuration"
  "windowsSdkVersion=$WindowsSdkVersion"
  "desktopHostLib=$desktopHostLib"
  "generatedHeader=$generatedHeader"
  "vsDevCmd=$vsdev"
) | Out-File -Encoding utf8 (Join-Path $EvidenceDir "summary.txt")
Get-Content (Join-Path $EvidenceDir "summary.txt")
