import { readFileSync } from 'node:fs';

import { expect, test } from 'bun:test';

test('Windows host lifecycle workflow preserves the issue #5 proof slice', () => {
  const workflow = readFileSync(
    new URL(
      '../../../../.github/workflows/windows-host-lifecycle.yml',
      import.meta.url
    ),
    'utf8'
  );
  const lifecycleHarness = readFileSync(
    new URL(
      '../../../../crates/servokit-host-desktop/tests/windows_native_lifecycle.cpp',
      import.meta.url
    ),
    'utf8'
  );
  const cBoundaryHarness = readFileSync(
    new URL(
      '../../../../crates/servokit-host-desktop/tests/c_boundary.rs',
      import.meta.url
    ),
    'utf8'
  );
  const embedderManifest = readFileSync(
    new URL('../../../../crates/servokit-embedder/Cargo.toml', import.meta.url),
    'utf8'
  );
  const lifecycleScript = readFileSync(
    new URL('../../../../scripts/windows-host-lifecycle.ps1', import.meta.url),
    'utf8'
  );
  const desktopSmokeScript = readFileSync(
    new URL('../../../../scripts/windows-desktop-smoke.ps1', import.meta.url),
    'utf8'
  );
  const readinessChecks = readFileSync(
    new URL('../../../../docs/readiness-checks.md', import.meta.url),
    'utf8'
  );
  const desktopWinitMain = readFileSync(
    new URL('../../../../examples/desktop-winit/src/main.rs', import.meta.url),
    'utf8'
  );
  const desktopWinitSmoke = readFileSync(
    new URL('../../../../examples/desktop-winit/src/smoke.rs', import.meta.url),
    'utf8'
  );
  const desktopWinitDocs = readFileSync(
    new URL('../../../../docs/desktop-winit.md', import.meta.url),
    'utf8'
  );

  expect(workflow).toContain('runs-on: windows-latest');
  expect(workflow).toContain('workflow_dispatch:');
  expect(workflow).toContain('pull_request:');
  expect(workflow).toContain('"crates/servokit-host-desktop/**"');
  expect(workflow).toContain('CARGO_BUILD_TARGET: x86_64-pc-windows-msvc');
  expect(workflow).toContain('./scripts/windows-host-lifecycle.ps1');
  expect(workflow).toContain(
    '-EvidenceDir "$env:RUNNER_TEMP\\servokit-windows-evidence"'
  );
  expect(workflow).toContain(
    'ServoKit-Windows-host-lifecycle-evidence'
  );
  expect(workflow).not.toContain('react-native-windows');
  expect(workflow).not.toContain('react_native_windows');

  expect(lifecycleScript).toContain('vswhere.exe');
  expect(lifecycleScript).toContain('This validation slice must run on Windows');
  expect(lifecycleScript).toContain(
    'Microsoft.VisualStudio.Component.VC.Tools.x86.x64'
  );
  expect(lifecycleScript).toContain('VsDevCmd.bat');
  expect(lifecycleScript).toContain('SERVOKIT_VSDEVCMD');
  expect(lifecycleScript).toContain('Resolve-ClangBin');
  expect(lifecycleScript).toContain('LLVM\\bin\\clang-cl.exe');
  expect(lifecycleScript).toContain('LLVM\\bin\\lld-link.exe');
  expect(lifecycleScript).toContain('$env:PATH = "$clangBin;$env:PATH"');
  expect(lifecycleScript).toContain('$env:CC = Join-Path $clangBin "clang-cl.exe"');
  expect(lifecycleScript).toContain('$env:CXX = $env:CC');
  expect(lifecycleScript).toContain('clangBin=$clangBin');
  expect(lifecycleScript).toContain('Resolve-Python3');
  expect(lifecycleScript).toContain(
    '*\\Microsoft\\WindowsApps\\python.exe'
  );
  expect(lifecycleScript).toContain('$env:PYTHON3 = $python3');
  expect(lifecycleScript).toContain('$env:PATH = "$pythonBin;$env:PATH"');
  expect(lifecycleScript).toContain('python3=$python3');
  expect(lifecycleScript).toContain('visual-studio.txt');
  expect(lifecycleScript).toContain('CARGO_TERM_COLOR');
  expect(lifecycleScript).toContain('RUST_BACKTRACE');
  expect(lifecycleScript).toContain('CARGO_TARGET_DIR');
  expect(lifecycleScript).toContain('servokit-target');
  expect(lifecycleScript).toContain('cargoTargetDir=$cargoTargetDir');
  expect(lifecycleScript).toContain('-arch=x64 -host_arch=x64');
  expect(lifecycleScript).toContain('$resolvedRepoRoot.ProviderPath');
  expect(lifecycleScript).toContain(
    '$RepoRoot = [System.IO.Path]::GetFullPath($RepoRoot)'
  );
  expect(lifecycleScript).toContain('$env:SERVOKIT_REPO_ROOT = $RepoRoot');
  expect(lifecycleScript).toContain(
    'servokitRepoRoot=$env:SERVOKIT_REPO_ROOT'
  );
  expect(lifecycleScript).toContain('safe.directory=$gitSafeDirectory');
  expect(lifecycleScript).toContain('core.fsmonitor=false');
  expect(lifecycleScript).toContain('core.autocrlf=false');
  expect(lifecycleScript).toContain('-ConfigArguments $gitConfigArgs');
  expect(lifecycleScript).toContain('pushd `"$RepoRoot`" >nul');
  expect(lifecycleScript).toContain(
    'Push-Location ([System.IO.Path]::GetTempPath())'
  );
  expect(lifecycleScript).toContain('SystemDrive');
  expect(lifecycleScript).toContain('Get-Content $LogPath -Tail 200');
  expect(lifecycleScript).toContain('>nul && $metadataCommand');
  expect(lifecycleScript).toContain('host: x86_64-pc-windows-msvc');
  expect(lifecycleScript).toContain(
    'Invoke-RepoGit -ConfigArguments $gitConfigArgs -Arguments @("rev-parse", "HEAD")'
  );
  expect(lifecycleScript).toContain(
    'Invoke-RepoGit -ConfigArguments $gitConfigArgs -Arguments @("status", "--short", "--branch", "--ignore-submodules=dirty")'
  );
  expect(lifecycleScript).toContain(
    'Invoke-RepoGit -ConfigArguments $gitConfigArgs -Arguments @("diff", "--ignore-submodules=dirty", "HEAD", "--stat")'
  );
  expect(lifecycleScript).toContain('git-diff-stat.txt');
  expect(lifecycleScript).toContain('git-diff-name-status.txt');
  expect(lifecycleScript).toContain(
    'Invoke-RepoGit -ConfigArguments $gitConfigArgs -Arguments @("submodule", "status", "--recursive")'
  );
  expect(lifecycleScript).toContain('powershell.txt');
  expect(lifecycleScript).toContain('source-hashes.txt');
  expect(lifecycleScript).toContain('Get-FileHash');
  expect(lifecycleScript).toContain('scripts/windows-desktop-smoke.ps1');
  expect(lifecycleScript).toContain('where rustc');
  expect(lifecycleScript).toContain('where cargo');
  expect(lifecycleScript).toContain('where rustup');
  expect(lifecycleScript).toContain('environment.txt');
  expect(lifecycleScript).toContain('commands.txt');
  expect(lifecycleScript).toContain('summary.txt');
  expect(lifecycleScript).toContain(
    'cargo metadata --manifest-path crates/Cargo.toml --locked --format-version 1'
  );
  expect(lifecycleScript).toContain('cargo-metadata.json');
  expect(lifecycleScript).toContain('servo-version.txt');
  expect(lifecycleScript).toContain(
    'cargo test --manifest-path crates/Cargo.toml -p servokit-host-desktop --test c_boundary --release --locked'
  );
  expect(lifecycleScript).toContain(
    'cargo check --manifest-path examples/desktop-winit/Cargo.toml --locked'
  );
  expect(lifecycleScript).not.toContain('react-native-windows');
  expect(lifecycleScript).not.toContain('react_native_windows');
  expect(lifecycleScript).not.toContain('Microsoft Visual Studio\\2022\\Enterprise');

  expect(cBoundaryHarness).toContain('let profile = if cfg!(debug_assertions)');
  expect(cBoundaryHarness).toContain('env::var_os("SERVOKIT_REPO_ROOT")');
  expect(cBoundaryHarness).toContain('nested_cargo_working_dir(workspace_dir)');
  expect(cBoundaryHarness).toContain('env::temp_dir()');
  expect(cBoundaryHarness).toContain('build.arg("--release");');
  expect(cBoundaryHarness).toContain('.env("CARGO_TERM_COLOR", "never")');
  expect(cBoundaryHarness).toContain('clang-cl.exe');
  expect(cBoundaryHarness).toContain('/EHsc');
  expect(cBoundaryHarness).toContain('/MD');
  expect(cBoundaryHarness).toContain('-fuse-ld=lld');
  expect(cBoundaryHarness).toContain('windows_x86_64_msvc');
  expect(cBoundaryHarness).toContain('starts_with("mozangle-")');
  expect(cBoundaryHarness).toContain('libEGL.lib');
  expect(cBoundaryHarness).toContain('libGLESv2.lib');
  expect(cBoundaryHarness).toContain('libEGL.dll');
  expect(cBoundaryHarness).toContain('libGLESv2.dll');
  expect(cBoundaryHarness).toContain('run_with_timeout');
  expect(cBoundaryHarness).toContain('Duration::from_secs(60)');
  expect(cBoundaryHarness).toContain('/LIBPATH:');
  expect(cBoundaryHarness).toContain('target_dir.join(target).join(profile)');
  expect(cBoundaryHarness).toContain('target_dir.join(profile)');

  expect(embedderManifest).toContain(
    "[target.'cfg(target_os = \"windows\")'.dependencies]"
  );
  expect(embedderManifest).toContain(
    'servo = { workspace = true, optional = true, features = ["no-wgl"] }'
  );

  expect(desktopSmokeScript).toContain('[switch]$Attended');
  expect(desktopSmokeScript).toContain('This smoke must run on Windows');
  expect(desktopSmokeScript).toContain('Re-run with -Attended');
  expect(desktopSmokeScript).toContain('servokit-windows-desktop-smoke');
  expect(desktopSmokeScript).toContain('examples/fixtures/smoke/index.html');
  expect(desktopSmokeScript).toContain('CARGO_BUILD_TARGET');
  expect(desktopSmokeScript).toContain('x86_64-pc-windows-msvc');
  expect(desktopSmokeScript).toContain('SERVOKIT_VSDEVCMD');
  expect(desktopSmokeScript).toContain('vswhere.exe');
  expect(desktopSmokeScript).toContain('$resolvedRepoRoot.ProviderPath');
  expect(desktopSmokeScript).toContain('$env:SERVOKIT_REPO_ROOT = $RepoRoot');
  expect(desktopSmokeScript).toContain('CARGO_TARGET_DIR');
  expect(desktopSmokeScript).toContain('cargoTargetDir=$cargoTargetDir');
  expect(desktopSmokeScript).toContain('safe.directory=$gitSafeDirectory');
  expect(desktopSmokeScript).toContain('core.fsmonitor=false');
  expect(desktopSmokeScript).toContain('core.autocrlf=false');
  expect(desktopSmokeScript).toContain('pushd `"$RepoRoot`" >nul');
  expect(desktopSmokeScript).toContain(
    'Push-Location ([System.IO.Path]::GetTempPath())'
  );
  expect(desktopSmokeScript).toContain(
    'Invoke-RepoGit -Arguments @("rev-parse", "HEAD")'
  );
  expect(desktopSmokeScript).toContain('Start-Process');
  expect(desktopSmokeScript).toContain('http.server');
  expect(desktopSmokeScript).toContain('desktop-winit-smoke.txt');
  expect(desktopSmokeScript).toContain(
    'cargo run --release --locked --manifest-path examples/desktop-winit/Cargo.toml -- --smoke'
  );
  expect(desktopSmokeScript).toContain('smoke result=pass');
  expect(desktopSmokeScript).toContain('input_probe=complete');
  expect(desktopSmokeScript).toContain('surface_attach_count=2');
  expect(desktopSmokeScript).toContain('surface_detach_count=2');
  expect(desktopSmokeScript).toContain('Stop-Process');
  expect(desktopSmokeScript).not.toContain('react-native-windows');
  expect(desktopSmokeScript).not.toContain('react_native_windows');

  expect(lifecycleHarness).toContain(
    'attached resize rejects stale token'
  );
  expect(lifecycleHarness).toContain('#define NOMINMAX');
  expect(lifecycleHarness).toContain('stage=%s');
  expect(lifecycleHarness).toContain(
    'TerminateProcess(GetCurrentProcess(), exit_code)'
  );
  expect(lifecycleHarness).toContain(
    'attached reattach rejects stale token'
  );
  expect(lifecycleHarness).toContain(
    'destroyed host rejects stale token'
  );
  expect(lifecycleHarness).toContain('rejected_generation != 0');

  expect(desktopWinitMain).toContain('run_smoke_reattach_cycle');
  expect(desktopWinitMain).toContain('run_smoke_input_probe');
  expect(desktopWinitMain).toContain('smoke_input_probe_events');
  expect(desktopWinitMain).toContain('HostInputEvent::Focus');
  expect(desktopWinitMain).toContain('PointerInputEvent::wheel');
  expect(desktopWinitMain).toContain('KeyboardInputEvent');
  expect(desktopWinitMain).toContain('HostInputEvent::ImeCommit');
  expect(desktopWinitMain).toContain('detach_surface(webview)');
  expect(desktopWinitMain).toContain('attach_surface_with_viewport');
  expect(desktopWinitSmoke).toContain('should_run_input_probe');
  expect(desktopWinitSmoke).toContain('smoke action=input-probe');
  expect(desktopWinitSmoke).toContain('smoke action=input-probe-complete');
  expect(desktopWinitSmoke).toContain('input_probe={}');
  expect(desktopWinitSmoke).toContain('should_run_reattach_cycle');
  expect(desktopWinitSmoke).toContain('smoke action=reattach-cycle');
  expect(desktopWinitSmoke).toContain('surface_attach_count >= 1');
  expect(desktopWinitSmoke).toContain('surface_attach_count >= 2');
  expect(desktopWinitSmoke).toContain('surface_detach_count >= 1');
  expect(desktopWinitSmoke).toContain(
    'waiting for load completion, input probe, and reattach'
  );
  expect(desktopWinitDocs).toContain('smoke action=input-probe');
  expect(desktopWinitDocs).toContain('input_probe=complete');
  expect(desktopWinitDocs).toContain('reattach=complete');
  expect(desktopWinitDocs).toContain('surface_detach_count=2');
  expect(desktopWinitDocs).toContain('scripts/windows-host-lifecycle.ps1');
  expect(desktopWinitDocs).toContain('scripts/windows-desktop-smoke.ps1 -Attended');

  expect(readinessChecks).toContain(
    '.github/workflows/windows-host-lifecycle.yml'
  );
  expect(readinessChecks).toContain('scripts/windows-host-lifecycle.ps1');
  expect(readinessChecks).toContain('scripts/windows-desktop-smoke.ps1 -Attended');
  expect(readinessChecks).toContain('source file hashes');
  expect(readinessChecks).toContain('input_probe=complete');
  expect(readinessChecks).toMatch(
    /React Native\s+Windows, binary distribution, GPU surfaces, ARM64 Windows/
  );
});
