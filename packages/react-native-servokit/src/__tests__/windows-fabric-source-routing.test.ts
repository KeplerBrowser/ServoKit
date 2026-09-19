import { readFileSync } from 'node:fs';

import { expect, test } from 'bun:test';

function readPackageFile(relativePath: string): string {
  return readFileSync(new URL(`../../${relativePath}`, import.meta.url), 'utf8');
}

test('Windows package metadata advertises only the source-side Fabric adapter', () => {
  const packageJson = JSON.parse(readPackageFile('package.json')) as {
    files: string[];
    peerDependencies: Record<string, string>;
    peerDependenciesMeta?: Record<string, { optional?: boolean }>;
    codegenConfig: {
      type: string;
      windows?: unknown;
    };
  };
  const examplePackageJson = JSON.parse(readPackageFile('example/package.json')) as {
    dependencies: Record<string, string>;
    devDependencies: Record<string, string>;
  };
  const reactNativeConfig = readPackageFile('react-native.config.js');
  const packageVerifier = readPackageFile('scripts/validate-packed-consumer.mjs');
  const exampleApp = readPackageFile('example/src/App.tsx');
  const autolinkChecker = readPackageFile(
    'scripts/check-windows-autolink-config.mjs'
  );

  expect(packageJson.files).toContain('windows');
  expect(packageJson.files).not.toContain('windows/servokit_host_desktop.dll');
  expect(packageJson.peerDependencies['react-native-windows']).toBe('*');
  expect(packageJson.peerDependenciesMeta?.['react-native-windows']?.optional).toBe(
    true
  );
  expect(packageJson.codegenConfig.type).toBe('all');
  expect(packageJson.codegenConfig.windows).toEqual({
    namespace: 'ServoKitCodegen',
    generators: ['componentsWindows', 'modulesWindows'],
    outputDirectory: 'windows/ServoKit/codegen',
    separateDataTypes: true,
  });
  expect(examplePackageJson.dependencies['react-native-windows']).toBe(
    '0.85.0-preview.1'
  );
  expect(examplePackageJson.devDependencies['react-native-windows']).toBeUndefined();
  expect(exampleApp).toContain('const fixtureServerBaseUrl = "http://127.0.0.1:8481"');
  expect(exampleApp).toContain('const smokeFixturesUrl = "http://127.0.0.1:8481/smoke/index.html"');
  expect(exampleApp).toContain('Platform.OS === "ios" ? homeUrl : smokeFixturesUrl');
  expect(exampleApp).toContain('onUrlChanged');
  expect(exampleApp).toContain('onLoadStatusChanged');
  expect(exampleApp).toContain('onHistoryChanged');
  expect(exampleApp).toContain('onFocusChanged');
  expect(exampleApp).toContain('onShouldStartLoadWithRequest');
  expect(exampleApp).toContain('onJavaScriptDialog');
  expect(exampleApp).toContain('evaluateJavaScript');
  expect(exampleApp).toContain('goBack()');
  expect(exampleApp).toContain('goForward()');
  expect(exampleApp).toContain('reload()');
  expect(exampleApp).toContain('focus()');
  expect(exampleApp).toContain('blur()');
  expect(exampleApp).toContain('setServoViewGeneration');
  expect(exampleApp).toContain('action.recycle-view');

  expect(reactNativeConfig).toContain("sourceDir: 'windows'");
  expect(reactNativeConfig).toContain("solutionFile: 'ServoKit.sln'");
  expect(reactNativeConfig).toContain("projectFile: 'ServoKit\\\\ServoKit.vcxproj'");
  expect(reactNativeConfig).toContain('directDependency: true');
  expect(reactNativeConfig).not.toContain('windows: null');
  expect(packageVerifier).toContain('isUnexpectedWindowsBinary');
  expect(packageVerifier).toContain('unexpected Windows native binaries');
  expect(autolinkChecker).toContain('process.platform !== "win32"');
  expect(autolinkChecker).toContain('"react-native.exe", "react-native.cmd"');
  expect(autolinkChecker).toContain('react-native-windows as a dependency');
  expect(autolinkChecker).toContain(
    'react-native config did not expose the ServoKit Windows dependency'
  );
  expect(autolinkChecker).toContain('React Native Windows did not register');
  expect(autolinkChecker).toContain('ServoKit/ServoKit.vcxproj');
  expect(autolinkChecker).toContain('ServoKit::ReactPackageProvider');
  expect(autolinkChecker).toContain('autolink-windows');
  expect(autolinkChecker).toContain('run-windows');
  expect(autolinkChecker).toContain('--require-windows-project');
  expect(autolinkChecker).toContain('config.project?.windows');
  expect(autolinkChecker).toContain(
    'React Native Windows app project solutionFile was not resolved'
  );
  expect(autolinkChecker).toContain(
    'React Native Windows app project projectGuid was not resolved'
  );

  for (const file of [
    'windows/ServoKit.sln',
    'windows/ServoKit/ServoKit.vcxproj',
    'windows/ServoKit/ServoView.cpp',
    'windows/ServoKit/ServoView.h',
    'windows/ServoKit/codegen/react/components/ServoViewSpec/ServoView.g.h',
    'windows/ServoKit/ReactPackageProvider.cpp',
    'windows/ServoKit/ReactPackageProvider.idl',
  ]) {
    expect(packageVerifier).toContain(file);
  }
  expect(packageVerifier).not.toContain('servokit_host_desktop.dll');
});

test('Windows Fabric component routes ServoView through the desktop host boundary', () => {
  const props = readPackageFile('windows/ServoKit/PropertySheet.props');
  const project = readPackageFile('windows/ServoKit/ServoKit.vcxproj');
  const nugetConfig = readPackageFile('windows/NuGet.Config');
  const provider = readPackageFile('windows/ServoKit/ReactPackageProvider.cpp');
  const moduleDefinition = readPackageFile('windows/ServoKit/ServoKit.def');
  const providerIdl = readPackageFile(
    'windows/ServoKit/ReactPackageProvider.idl'
  );
  const header = readPackageFile('windows/ServoKit/ServoView.h');
  const source = readPackageFile('windows/ServoKit/ServoView.cpp');
  const wrapper = readPackageFile('src/ServoView.tsx');
  const generatedHeader = readPackageFile(
    'windows/ServoKit/codegen/react/components/ServoViewSpec/ServoView.g.h'
  );
  const validationScript = readFileSync(
    new URL('../../../../scripts/windows-rnw-source.ps1', import.meta.url),
    'utf8'
  );
  const rustLinkScript = readFileSync(
    new URL(
      '../../../../scripts/windows-rust-staticlib-link.ps1',
      import.meta.url
    ),
    'utf8'
  );
  const smokeScript = readFileSync(
    new URL('../../../../scripts/windows-rnw-smoke.ps1', import.meta.url),
    'utf8'
  );
  const validationWorkflow = readFileSync(
    new URL(
      '../../../../.github/workflows/windows-rnw-source.yml',
      import.meta.url
    ),
    'utf8'
  );

  expect(props).toContain('ServokitDesktopHostIncludeDir');
  expect(props).toContain('ServokitCargoTargetDir');
  expect(props).toContain('ServokitDesktopHostLibDir');
  expect(props).toContain('ServokitDesktopHostLinkProps');
  expect(props).toContain('crates\\servokit-host-desktop\\include');
  expect(props).toContain('$(CARGO_TARGET_DIR)');
  expect(props).toContain(
    '$(ServokitCargoTargetDir)\\x86_64-pc-windows-msvc\\$(ServokitDesktopHostProfile)'
  );

  expect(project).toContain('Microsoft.ReactNative.Composition.CppLib.props');
  expect(project).toContain('Microsoft.ReactNative.Composition.CppLib.targets');
  expect(project).toContain('<Keyword>Win32Proj</Keyword>');
  expect(project).toContain('<AppxPackage>false</AppxPackage>');
  expect(project).toContain('<PlatformToolset>v143</PlatformToolset>');
  expect(project).not.toContain('<ApplicationType>Windows Store</ApplicationType>');
  expect(project).not.toContain('packages.config');
  expect(project).toContain('RNW_NEW_ARCH');
  expect(project).toContain('$(ServokitDesktopHostIncludeDir)');
  expect(project).toContain('$(ServokitDesktopHostLibDir)');
  expect(project).toContain('servokit_host_desktop.lib');
  expect(project).toContain('$(ServokitDesktopHostNativeLibraries)');
  expect(project).toContain('$(ServokitDesktopHostWindowsImportLibDir)');
  expect(project).toContain('$(ServokitDesktopHostAngleLibDir)');
  expect(project).toContain('$(ServokitDesktopHostAngleLibDir)\\libEGL.dll');
  expect(project).toContain('$(ServokitDesktopHostAngleLibDir)\\libGLESv2.dll');
  expect(project).not.toContain('<DeploymentContent>true</DeploymentContent>');
  expect(project).toContain('<CopyToOutputDirectory>PreserveNewest</CopyToOutputDirectory>');
  expect(project).toContain('$(ServokitDesktopHostLinkProps)');
  expect(project).toContain('EnsureServokitDesktopHost');
  expect(project).not.toContain('Microsoft.ReactNative.Uwp.CppLib');
  expect(nugetConfig).toContain(
    'https://pkgs.dev.azure.com/ms/react-native/_packaging/react-native-public/nuget/v3/index.json'
  );
  expect(nugetConfig).toContain('https://api.nuget.org/v3/index.json');

  for (const evidence of [
    'native-static-libs:',
    'windows_x86_64_msvc',
    'mozangle-*',
    'libEGL.lib',
    'libGLESv2.lib',
    'libEGL.dll',
    'libGLESv2.dll',
    'angleRuntimeLibraries=libEGL.dll;libGLESv2.dll',
    'ServokitDesktopHostWindowsImportLibDir',
    'ServokitDesktopHostAngleLibDir',
    'ServokitDesktopHostNativeLibraries',
    'servokit_host_desktop.props',
  ]) {
    expect(rustLinkScript).toContain(evidence);
  }

  expect(provider).toContain('RegisterServoViewNativeComponent(packageBuilder)');
  expect(moduleDefinition).toContain('DllCanUnloadNow = WINRT_CanUnloadNow');
  expect(moduleDefinition).toContain(
    'DllGetActivationFactory = WINRT_GetActivationFactory'
  );
  expect(moduleDefinition).not.toContain('DllGetClassObject');
  expect(providerIdl).toMatch(/ReactPackageProvider\(\);\r?\n\s*};/);
  expect(header).toContain(
    '#include "codegen/react/components/ServoViewSpec/ServoView.g.h"'
  );
  expect(header).toContain('ServoKitCodegen::BaseServoView<ServoViewComponentView>');
  expect(header).toContain('<servokit_desktop_private.h>');
  expect(generatedHeader).toContain(
    'struct ServoViewProps : winrt::implements<ServoViewProps'
  );
  expect(generatedHeader).toContain('struct ServoViewEventEmitter');
  expect(generatedHeader).toContain('struct BaseServoView');
  expect(generatedHeader).toContain('RegisterServoViewNativeComponent');
  expect(generatedHeader).toContain('HandleSendControllerCommandCommand');
  expect(source).toContain('ServoKitCodegen::ServoViewSpec_onUrlChanged');
  expect(source).toContain(
    'ServoKitCodegen::ServoViewSpec_onJavaScriptEvaluationResult'
  );
  expect(source).toContain(
    'ServoKitCodegen::ServoViewSpec_onShouldStartLoadWithRequestRequested'
  );
  expect(source).toContain(
    'ServoKitCodegen::ServoViewSpec_onCreateNewWebViewRequested'
  );
  expect(source).toContain(
    'ServoKitCodegen::ServoViewSpec_onJavaScriptDialogRequested'
  );
  expect(source).toContain(
    'ServoKitCodegen::ServoViewSpec_onJavaScriptDialogDismissed'
  );
  expect(source).not.toContain('ServoKitCodegen::ServoView_On');
  expect(source).toContain('emitter->onUrlChanged(std::move(value));');
  expect(source).toContain(
    'emitter->onJavaScriptEvaluationResult(std::move(value));'
  );
  expect(source).toContain(
    'emitter->onShouldStartLoadWithRequestRequested(std::move(value));'
  );
  expect(source).toContain(
    'emitter->onCreateNewWebViewRequested(std::move(value));'
  );
  expect(source).toContain(
    'emitter->onJavaScriptDialogRequested(std::move(value));'
  );
  expect(source).toContain(
    'emitter->onJavaScriptDialogDismissed(std::move(value));'
  );
  expect(source).not.toContain('emitter->onUrlChanged(value);');

  for (const symbol of [
    'ServoKitCodegen::RegisterServoViewNativeComponent',
    'HandleSendControllerCommandCommand',
    'servokit_desktop_private_create',
    'servokit_desktop_private_attach',
    'servokit_desktop_private_update_viewport',
    'servokit_desktop_private_detach',
    'servokit_desktop_private_destroy',
    'servokit_desktop_private_dispatch_controller_command',
    'DispatchControllerCommandJson',
    'ResolveNavigationRequest',
    'resolveNavigationRequest',
    'ResolveSimpleDialog',
    'resolveSimpleDialog',
    'DismissContextMenu',
    'dismissContextMenu',
    'useReactNativeOnShouldStartLoadWithRequest.value_or(false)',
    'useReactNativeJavaScriptDialogs.value_or(false)',
    'servokit_desktop_private_dispatch_input',
    'servokit_desktop_private_pump',
    'servokit_desktop_private_drain_events',
    'SchedulePumpForToken',
    'm_ownerThreadId = GetCurrentThreadId();',
    'm_uiDispatcher = reactContext.UIDispatcher();',
    'GetCurrentThreadId() == m_ownerThreadId',
    'dispatcher.Post',
    'get_weak()',
    'ReactCoreInjection::GetTopLevelWindowId',
    'GetDpiForWindow',
    'JsonObject command',
  ]) {
    expect(source).toContain(symbol);
  }
  expect(source).not.toContain('dispatcher.HasThreadAccess()');
  expect(source).toContain('[weakThis = get_weak()](');
  expect(source).not.toContain('[weakThis = get_weak(), view]');
  expect(source).toContain(
    'sender.try_as<winrt::Microsoft::ReactNative::ComponentView>()'
  );
  expect(source).toContain('m_loadedUrl = value.url;');
  expect(source).toContain('L"Microsoft.UI.Content.DesktopChildSiteBridge"');
  expect(source).toContain('return topLevelHwnd;');
  expect(source).toContain('GetWindowLongPtrW(islandHwnd, GWL_STYLE) | WS_CLIPSIBLINGS');
  expect(source).toContain('view->m_keyboardCharacters.find(scanCode)');
  expect(source).toContain('view->DispatchKeyboardCharacter(text, false, action)');
  expect(source).toContain('ReleaseKeyboardCharacters();');
  expect(source).toContain("wparam == L' ' && view->m_spaceKeyDown");
  expect(source).toContain('input.delta_y = -delta;');
  expect(source).toContain('ComponentViewFeatures::NativeBorder |');
  expect(source).toContain('ComponentViewFeatures::Background');
  expect(source).toContain('HWND_TOP');
  expect(source).not.toContain('SWP_NOZORDER');
  expect(header).toContain(
    'ServokitDesktopPrivateStatus DispatchControllerCommandJson'
  );
  expect(source).toContain(
    'DispatchControllerCommandJson(commandJson) == SERVOKIT_DESKTOP_PRIVATE_OK'
  );
  const attachCall = source.indexOf('servokit_desktop_private_attach(');
  expect(attachCall).toBeGreaterThan(-1);
  const detachedLoad = source.indexOf('LoadUrlIfNeeded(true)');
  expect(detachedLoad).toBeGreaterThan(-1);
  expect(detachedLoad).toBeLessThan(attachCall);
  expect(source.indexOf('LoadUrlIfNeeded();', attachCall)).toBeGreaterThan(attachCall);

  for (const input of [
    'SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_MOVE',
    'SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_BUTTON',
    'SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_WHEEL',
    'SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_LEAVE',
    'SERVOKIT_DESKTOP_PRIVATE_INPUT_KEYBOARD',
    'SERVOKIT_DESKTOP_PRIVATE_INPUT_IME_COMMIT',
    'SERVOKIT_DESKTOP_PRIVATE_INPUT_FOCUS',
    'SERVOKIT_DESKTOP_PRIVATE_KEY_NAMED',
    'SERVOKIT_DESKTOP_PRIVATE_KEY_CHARACTER',
  ]) {
    expect(source).toContain(input);
  }
  expect(source).toContain('TRACKMOUSEEVENT track{};');
  expect(source).toContain('TrackMouseEvent(&track);');
  expect(source).toContain('name == "navigationRequested"');
  expect(source).toContain('name == "popupRequested"');
  expect(source).toContain('name == "simpleDialogRequested"');
  expect(source).toContain('name == "contextMenuRequested"');
  expect(source).toContain('} else if (!emitter) {');
  expect(source).toContain('ResolveNavigationRequest(navigationId, true);');
  expect(source).toContain('ResolveSimpleDialog(dialogId, false, std::nullopt);');
  expect(source).toContain('DismissContextMenu(JsonString(payload, L"contextMenuId"));');
  expect(wrapper).toContain('function dismissContextMenuCommandJson(contextMenuId: string)');
  expect(wrapper).toContain("createControllerCommandJson('dismissContextMenu', { contextMenuId })");
  expect(wrapper).toContain("Platform.OS === 'android' || Platform.OS === 'windows'");
  expect(wrapper).toContain('if (!shown) {');
  expect(wrapper).toContain('sendControllerCommand(dismissContextMenuCommandJson(contextMenuId));');
  const pch = readPackageFile('windows/ServoKit/pch.h');
  expect(pch).toContain('#define NOMINMAX');
  expect(pch).toContain('#include <utility>');
  expect(pch).toContain('#include <winrt/Windows.Foundation.Collections.h>');

  expect(source).not.toContain('GetType()');
  expect(source).not.toContain('TODO');
  expect(source).not.toContain('servokit_host_desktop.dll');

  for (const evidence of [
    'servokit-windows-rnw-source',
    '$resolvedRepoRoot.ProviderPath',
    '$env:SERVOKIT_REPO_ROOT = $RepoRoot',
    'servokit-target',
    '$cargoTargetDir = Join-Path $RepoRoot "target"',
    '$env:CARGO_TARGET_DIR = $cargoTargetDir',
    'pushd `"$WorkingDirectory`"',
    'core.fsmonitor=false',
    'core.autocrlf=false',
    'safe.directory=$gitSafeDirectory',
    'Resolve-ClangBin',
    '$env:CC = Join-Path $clangBin "clang-cl.exe"',
    '$env:CXX = $env:CC',
    'Resolve-Python3',
    '$env:PYTHON3 = $python3',
    'Resolve-WindowsSdkVersion',
    '[Version]"10.0.22621.0"',
    '"/p:WindowsTargetPlatformVersion=$WindowsSdkVersion"',
    '"windowsSdkVersion=$WindowsSdkVersion"',
    'CARGO_BUILD_TARGET must be x86_64-pc-windows-msvc',
    'servokit-host-desktop',
    '.github/workflows/windows-rnw-source.yml',
    'scripts/check-windows-autolink-config.mjs',
    'example/package.json',
    'rnwAutolinkConfig',
    '@react-native-windows/codegen/bin.js',
    'servokit-rnw-codegen-',
    '[System.IO.File]::WriteAllText(',
    '--test',
    'windows/ServoKit/codegen/react/components/ServoViewSpec/ServoView.g.h',
    'msbuild',
    '"/restore"',
    '"/p:RestorePackagesConfig=true"',
    '$cargoBuildArgs = @(',
    'if ($Configuration -eq "Release")',
    '$cargoBuildArgs += "--release"',
    '$cargoBuildArgs += @("--", "--print", "native-static-libs")',
    '$cargoBuildCommand = "cargo " + (Join-ProcessArguments -Arguments $cargoBuildArgs)',
    'scripts/windows-rust-staticlib-link.ps1',
    'servokit_host_desktop.props',
    'servokit-host-desktop-link.txt',
    'ServokitDesktopHostLibDir',
    'rnw-autolink-config.txt',
    'rnw-codegen.txt',
    'rnw-msbuild.txt',
    'summary.txt',
  ]) {
    expect(validationScript).toContain(evidence);
  }

  for (const evidence of [
    'name: Windows RNW Source',
    'workflow_dispatch:',
    'pull_request:',
    'configuration:',
    'runs-on: windows-latest',
    'CARGO_BUILD_TARGET: x86_64-pc-windows-msvc',
    'oven-sh/setup-bun@0c5077e51419868618aeaa5fe8019c62421857d6',
    'bun-version: 1.3.13',
    'bun install --frozen-lockfile --ignore-scripts',
    'bun run --cwd packages/react-native-servokit typecheck',
    'windows-fabric-source-routing.test.ts',
    'package-platform-contract.test.ts',
    './scripts/windows-rnw-source.ps1',
    "inputs.configuration || 'Release'",
    '-Configuration $configuration',
    '-EvidenceDir "$env:RUNNER_TEMP\\servokit-windows-rnw-source"',
    'ServoKit-Windows-RNW-source-evidence',
  ]) {
    expect(validationWorkflow).toContain(evidence);
  }
  expect(validationWorkflow).not.toContain('scripts/windows-desktop-smoke.ps1');
  expect(validationWorkflow).not.toContain('servokit_host_desktop.dll');

  for (const evidence of [
    '[switch]$Attended',
    'This smoke must run on Windows',
    'Re-run with -Attended',
    'FixturePort must be in the range 1..65535',
    'MetroPort must be in the range 1..65535',
    'servokit-windows-rnw-smoke',
    'AppRoot must contain a React Native Windows app project',
    'Resolve-AppSmokeSource',
    'AppRoot must include a JS/TS smoke source that mounts ServoView',
    '$requiredMarkers = @(',
    'onShouldStartLoadWithRequest',
    'evaluateJavaScript',
    'setServoViewGeneration',
    'action.recycle-view',
    'Missing markers',
    '$missingMarkers = @($requiredMarkers',
    'appSmokeSource=$appSmokeSource',
    'CARGO_BUILD_TARGET must be x86_64-pc-windows-msvc',
    'scripts/check-windows-autolink-config.mjs',
    '--require-windows-project',
    'servokit-host-desktop',
    'scripts/windows-rust-staticlib-link.ps1',
    '$cargoBuildArgs += @("--", "--print", "native-static-libs")',
    'servokit-host-desktop-link.txt',
    'http.server',
    'fixture-server.stdout.txt',
    'metro.stdout.txt',
    'run-windows',
    '--arch',
    'x64',
    '--no-packager',
    '--msbuildprops',
    'PlatformToolset=v143',
    'ServokitDesktopHostIncludeDir',
    'ServokitDesktopHostLibDir',
    'rnw-run-windows.txt',
    'attended-checklist.txt',
    'attended-result.txt',
    'Verify the launched RNW app renders $smokeUrl',
    'type PASS',
    'status=attended-incomplete',
    'RNW smoke checklist was not confirmed with PASS',
    'status=attended-pass',
    'two recycle/remount cycles',
    'Stop-Process',
  ]) {
    expect(smokeScript).toContain(evidence);
  }
  expect(smokeScript).not.toContain('servokit_host_desktop.dll');
});
