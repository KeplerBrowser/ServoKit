import {
  chmodSync,
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  realpathSync,
  readdirSync,
  readFileSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { expect, test } from 'bun:test';

const darwinTest = process.platform === 'darwin' ? test : test.skip;

test('CocoaPods selects the platform Fabric source and macOS routes generated commands', () => {
  const packageJson = JSON.parse(
    readFileSync(new URL('../../package.json', import.meta.url), 'utf8')
  ) as { scripts: Record<string, string> };
  const podspec = readFileSync(
    new URL('../../ExplorerKit.podspec', import.meta.url),
    'utf8'
  );
  const macosPodspec = readFileSync(
    new URL('../../macos/ExplorerKitMacOS.podspec', import.meta.url),
    'utf8'
  );
  const podfile = readFileSync(
    new URL(
      '../../../../examples/react-native-macos-app/macos/Podfile',
      import.meta.url
    ),
    'utf8'
  );
  const iosPodfile = readFileSync(
    new URL('../../example/ios/Podfile', import.meta.url),
    'utf8'
  );
  const sourcePreparer = readFileSync(
    new URL('../../scripts/prepare-macos-source.mjs', import.meta.url),
    'utf8'
  );
  const source = readFileSync(
    new URL('../../macos/ExplorerView.mm', import.meta.url),
    'utf8'
  );
  const header = readFileSync(
    new URL(
      '../../../../crates/explorerkit-host-desktop/include/explorerkit_desktop_private.h',
      import.meta.url
    ),
    'utf8'
  );
  const artifactBuilder = readFileSync(
    new URL(
      '../../../../distribution/macos/build-xcframework.sh',
      import.meta.url
    ),
    'utf8'
  );
  const binaryPodspec = readFileSync(
    new URL(
      '../../../../distribution/macos/ExplorerKitMacOSBinary.podspec.template',
      import.meta.url
    ),
    'utf8'
  );
  const cargoManifest = readFileSync(
    new URL(
      '../../../../crates/explorerkit-host-desktop/Cargo.toml',
      import.meta.url
    ),
    'utf8'
  );
  const workflow = readFileSync(
    new URL(
      '../../../../.github/workflows/macos-xcframework.yml',
      import.meta.url
    ),
    'utf8'
  );

  expect(JSON.stringify(packageJson.scripts)).not.toContain(
    'prepare-macos-source'
  );
  expect(podspec).toContain('s.source_files = "ios/ExplorerView.mm"');
  expect(podspec).not.toContain('macOS');
  expect(podspec).not.toContain('EXPLORERKIT_BUILD_FROM_SOURCE');
  expect(macosPodspec).toContain('s.name         = "ExplorerKitMacOS"');
  expect(macosPodspec).toContain('s.source_files = "ExplorerView.mm"');
  expect(macosPodspec).toContain(
    's.dependency "ExplorerKitMacOSBinary", package["version"]'
  );
  expect(macosPodspec).toContain('ENV["EXPLORERKIT_BUILD_FROM_SOURCE"] == "1"');
  expect(macosPodspec).toContain(
    's.vendored_frameworks = "../.explorerkit-source/current/ExplorerKit.xcframework"'
  );
  expect(macosPodspec).not.toMatch(/\bsystem\s*\(/);
  expect(macosPodspec).not.toContain('build-xcframework');
  expect(macosPodspec).not.toContain('EXPLORERKIT_SOURCE_DIR');
  expect(macosPodspec).not.toContain('EXPLORERKIT_ARTIFACT_DIR');
  expect(macosPodspec).not.toContain('script_phase');
  expect(macosPodspec).not.toContain('prepare_command');
  expect(macosPodspec).not.toContain('postinstall');
  expect(macosPodspec).not.toContain('curl');
  expect(macosPodspec).not.toContain('download');
  expect(macosPodspec).not.toContain('OTHER_LIBTOOLFLAGS');
  expect(macosPodspec).not.toContain('libexplorerkit_host_desktop.a');
  expect(macosPodspec).not.toContain('${PODS_TARGET_SRCROOT}/crates/Cargo.toml');
  expect(podfile).toContain(
    "if ENV['EXPLORERKIT_BUILD_FROM_SOURCE'] == '1'"
  );
  expect(podfile).toContain(
    "scripts/prepare-macos-source.mjs"
  );
  expect(podfile).toContain(
    "elsif ENV['EXPLORERKIT_MACOS_BINARY_POD_DIR']"
  );
  expect(podfile).toContain(
    "pod 'ExplorerKitMacOS', :path => File.join(explorerkit_package_root, 'macos')"
  );
  expect(podfile.indexOf('prepare-macos-source.mjs')).toBeLessThan(
    podfile.indexOf('use_native_modules!')
  );
  expect(iosPodfile).not.toContain('prepare-macos-source');
  expect(sourcePreparer).toContain(
    "const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)))"
  );
  expect(sourcePreparer).toContain(
    "EXPLORERKIT_SOURCE_DIR is required when EXPLORERKIT_BUILD_FROM_SOURCE=1"
  );
  expect(sourcePreparer).toContain("EXPLORERKIT_FRAMEWORK_ONLY: '1'");
  expect(sourcePreparer).toContain("'/usr/bin/ditto'");
  expect(
    sourcePreparer.indexOf('writeFileSync(ownerMarker(directory)')
  ).toBeLessThan(sourcePreparer.indexOf('writeFileSync(pidMarker'));
  expect(sourcePreparer).not.toContain('postinstall');
  expect(source).toContain(
    '#import <ExplorerKit/explorerkit_desktop_private.h>'
  );
  expect(source).not.toContain('#include "explorerkit_desktop_private.h"');
  expect(source).toContain(`- (void)handleCommand:(const NSString *)commandName args:(const NSArray *)args
{
  RCTExplorerViewHandleCommand(self, commandName, args);
}`);
  expect(source).toContain(`emitter->onLoadStatusChanged({
        .status = ExplorerKitStdString(payload[@"status"]),
    });`);
  expect(source).toContain(`emitter->onCrashed({
        .reason = ExplorerKitStdString(payload[@"reason"]),
        .backtrace = ExplorerKitStdString(payload[@"backtrace"]),
    });`);
  expect(source).toContain(`emitter->onError({
        .code = ExplorerKitInt(payload[@"code"]),
        .message = ExplorerKitStdString(payload[@"message"]),
    });`);
  const exports = [
    ...new Set(header.match(/explorerkit_desktop_private_[a-z_]+/g) ?? []),
  ].sort();
  expect(exports).toEqual([
    'explorerkit_desktop_private_attach',
    'explorerkit_desktop_private_create',
    'explorerkit_desktop_private_destroy',
    'explorerkit_desktop_private_detach',
    'explorerkit_desktop_private_dispatch_controller_command',
    'explorerkit_desktop_private_dispatch_input',
    'explorerkit_desktop_private_drain_events',
    'explorerkit_desktop_private_pump',
    'explorerkit_desktop_private_update_viewport',
  ]);
  for (const symbol of exports) {
    expect(source).toContain(
      symbol === 'explorerkit_desktop_private_destroy'
        ? symbol
        : `${symbol}(`
    );
  }
  expect(source).toContain('attachmentGeneration != _attachmentGeneration');
  expect(source).toContain('event.hasPreciseScrollingDeltas');
  expect(source).toContain('point.y * scale');
  expect(source).not.toContain('NSHeight(self.bounds) - point.y');
  expect(source).toContain('input.delta_y = event.scrollingDeltaY');
  expect(source).not.toContain('input.delta_y = -event.scrollingDeltaY');
  expect(source).toContain('EXPLORERKIT_DESKTOP_PRIVATE_INPUT_IME_COMMIT');
  expect(source).toContain('_interpretedTextWasComposing');
  expect(source).toContain('} else if (!_compositionActive) {');
  expect(source).toContain('characterText:_interpretedText');
  expect(source).toContain('status == EXPLORERKIT_DESKTOP_PRIVATE_PANIC');
  expect(source).toContain(`if (_attachmentGeneration == 0) {
    [self loadURLIfNeededAllowingDetached:YES];
    if (_host == nullptr) {
      return;
    }`);
  expect(source).toContain(`_attachmentGeneration = generation;
      _viewport = viewport;
      [self pumpForToken:_token];
      [self loadURLIfNeeded];`);
  expect(source).toContain(`_url = [NSString stringWithUTF8String:viewProps.url.c_str()] ?: @"";
  [self syncSurface];`);
  expect(source).toContain(
    'if ((!allowDetached && _attachmentGeneration == 0) ||'
  );
  expect(source).toContain(
    'if ([self dispatchControllerCommand:command] == EXPLORERKIT_DESKTOP_PRIVATE_OK)'
  );
  expect(source).not.toContain(`if (_host != nullptr) {
    _loadedURL = [_url copy];
  }`);
  expect(source).not.toContain('SERVOKIT_DIAGNOSTIC_INITIAL_LOAD_PATH');
  expect(source).not.toContain('[SERVOKIT-LIFECYCLE]');
  expect(source).not.toContain('[SERVOKIT-WHEEL-239]');
  expect(source).toContain('ExplorerKitDestroyHost(');
  expect(cargoManifest).toContain('crate-type = ["rlib", "staticlib"]');
  expect(cargoManifest).not.toContain('"cdylib"');
  expect(artifactBuilder).toContain('export MOZJS_FROM_SOURCE=1');
  expect(artifactBuilder).toContain('export CARGO_ENCODED_RUSTFLAGS');
  expect(artifactBuilder).toContain(
    '"--remap-path-prefix=${HOME}=/home"'
  );
  expect(artifactBuilder).toContain(
    '"--remap-path-prefix=${cargo_home}=/cargo"'
  );
  expect(artifactBuilder).toContain(
    '"--remap-path-prefix=${root}=/src/explorerkit"'
  );
  expect(artifactBuilder).toContain(
    '"--remap-path-prefix=${target_dir}=/target"'
  );
  expect(artifactBuilder).toContain(
    '"-ffile-prefix-map=${HOME}=/home"'
  );
  expect(artifactBuilder).toContain(
    '"-ffile-prefix-map=${cargo_home}=/cargo"'
  );
  expect(artifactBuilder).toContain(
    '"-ffile-prefix-map=${root}=/src/explorerkit"'
  );
  expect(artifactBuilder).toContain(
    '"-ffile-prefix-map=${target_dir}=/target"'
  );
  expect(artifactBuilder).toContain(
    'export CXXFLAGS=" ${clang_remap_flags[*]}"'
  );
  expect(artifactBuilder).not.toContain('unset CXXFLAGS');
  expect(artifactBuilder).toContain(
    'for local_path in "${root}" "${target_dir}" "${cargo_home}" "${HOME}"; do'
  );
  expect(artifactBuilder).toContain(
    'if grep -aFq "${local_path}" "${binary}"; then'
  );
  expect(artifactBuilder).not.toContain(
    'strings -a "${binary}" | grep -Fq'
  );
  expect(artifactBuilder).toContain('CC_x86_64_apple_darwin=');
  expect(artifactBuilder).toContain('CXX_x86_64_apple_darwin=');
  expect(artifactBuilder).toContain('AR_x86_64_apple_darwin=');
  expect(artifactBuilder).toContain('mozjs-object-build-versions.txt');
  expect(artifactBuilder).toContain(
    'libexplorerkit_host_desktop.a'
  );
  expect(artifactBuilder).toContain(
    'if [ "${EXPLORERKIT_FRAMEWORK_ONLY:-0}" = "1" ]'
  );
  expect(artifactBuilder).not.toMatch(/\brm\s+-/);
  expect(artifactBuilder).toContain('xcrun clang');
  expect(artifactBuilder).toContain('-dynamiclib');
  expect(artifactBuilder).toContain(
    '@rpath/ExplorerKit.framework/Versions/A/ExplorerKit'
  );
  expect(artifactBuilder).toContain('xcodebuild -create-xcframework');
  expect(artifactBuilder).toContain('xcrun dsymutil');
  expect(artifactBuilder).toContain('create-sbom.mjs');
  expect(binaryPodspec).toContain(
    's.vendored_frameworks = "ExplorerKit.xcframework"'
  );
  expect(binaryPodspec).toContain(':sha256 => "__SHA256__"');
  expect(workflow).toContain(
    'sudo xcode-select -s "/Applications/Xcode_16.4.app"'
  );
  expect(workflow).toContain(
    'test "$(xcode-select --print-path)" = "/Applications/Xcode_16.4.app/Contents/Developer"'
  );
  expect(workflow).toContain(
    `test "$(xcodebuild -version | sed -n '1p')" = "Xcode 16.4"`
  );
  expect(source).toContain(`extern "C" Class<RCTComponentViewProtocol> ExplorerViewCls(void)
{
  return ExplorerView.class;
}`);
});

darwinTest('an isolated macOS CocoaPods resolution selects the repository pod and local binary pod', () => {
  const packageRoot = dirname(
    dirname(dirname(fileURLToPath(import.meta.url)))
  );
  const packageJson = JSON.parse(
    readFileSync(new URL('../../package.json', import.meta.url), 'utf8')
  ) as { version: string };
  const temporaryRoot = mkdtempSync(
    join(tmpdir(), 'explorerkit-macos-pod-resolution-')
  );
  const specRepository = join(temporaryRoot, 'specs.git');
  const binaryPod = join(temporaryRoot, 'ExplorerKitMacOSBinary');
  const xcframework = join(binaryPod, 'ExplorerKit.xcframework');
  const framework = join(
    xcframework,
    'macos-arm64',
    'ExplorerKit.framework'
  );
  const podfile = `source ${JSON.stringify(pathToFileURL(specRepository).href)}

install! 'cocoapods', :integrate_targets => false
platform :macos, '14.0'

class << ::Pod
  def install_modules_dependencies(_spec)
  end
end

target 'ExplorerKitMacOSProjection' do
  pod 'ExplorerKitMacOSBinary', :path => ${JSON.stringify(binaryPod)}
  pod 'ExplorerKitMacOS', :path => ${JSON.stringify(join(packageRoot, 'macos'))}
end
`;

  try {
    expect(Bun.spawnSync(['git', 'init', '--bare', specRepository]).success).toBe(
      true
    );
    mkdirSync(framework, { recursive: true });
    writeFileSync(
      join(xcframework, 'Info.plist'),
      `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>AvailableLibraries</key><array><dict>
<key>LibraryIdentifier</key><string>macos-arm64</string>
<key>LibraryPath</key><string>ExplorerKit.framework</string>
<key>SupportedArchitectures</key><array><string>arm64</string></array>
<key>SupportedPlatform</key><string>macos</string>
</dict></array>
<key>CFBundlePackageType</key><string>XFWK</string>
<key>XCFrameworkFormatVersion</key><string>1.0</string>
</dict></plist>
`
    );
    writeFileSync(
      join(framework, 'Info.plist'),
      `<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>ExplorerKit</string>
<key>CFBundleIdentifier</key><string>com.kepler.explorerkit.test</string>
<key>CFBundlePackageType</key><string>FMWK</string>
</dict></plist>
`
    );
    writeFileSync(join(framework, 'ExplorerKit'), '');
    writeFileSync(
      join(binaryPod, 'ExplorerKitMacOSBinary.podspec'),
      `Pod::Spec.new do |s|
  s.name = 'ExplorerKitMacOSBinary'
  s.version = '${packageJson.version}'
  s.summary = 'Local ExplorerKit macOS binary test stub'
  s.homepage = 'https://example.invalid'
  s.license = { :type => 'MIT' }
  s.author = 'ExplorerKit'
  s.source = { :git => 'https://example.invalid/ExplorerKit.git' }
  s.platform = :osx, '14.0'
  s.vendored_frameworks = 'ExplorerKit.xcframework'
end
`
    );
    writeFileSync(join(temporaryRoot, 'Podfile'), podfile);

    const result = Bun.spawnSync(
      ['pod', 'install', '--no-repo-update'],
      {
        cwd: temporaryRoot,
        env: {
          ...process.env,
          COCOAPODS_DISABLE_STATS: 'true',
          CP_HOME_DIR: join(temporaryRoot, 'cocoapods-home'),
        },
        stderr: 'inherit',
        stdout: 'inherit',
      }
    );

    expect(result.success).toBe(true);
    const lockfile = readFileSync(join(temporaryRoot, 'Podfile.lock'), 'utf8');
    expect(lockfile).toContain(`- ExplorerKitMacOS (${packageJson.version})`);
    expect(lockfile).toContain(
      `- ExplorerKitMacOSBinary (${packageJson.version})`
    );
    const podsProject = readFileSync(
      join(temporaryRoot, 'Pods', 'Pods.xcodeproj', 'project.pbxproj'),
      'utf8'
    );
    expect(podsProject).toContain('ExplorerView.mm');
    expect(podsProject).toContain('ExplorerKit.xcframework');
  } finally {
    rmSync(temporaryRoot, { recursive: true, force: true });
  }
});

test('macOS artifact builder rejects whitespace before launching Cargo', () => {
  const temporaryRoot = mkdtempSync(
    join(tmpdir(), 'explorerkit-macos-path-preflight-')
  );
  const fakeBin = join(temporaryRoot, 'bin');
  const cargoMarker = join(temporaryRoot, 'cargo-launched');
  const cargo = join(fakeBin, 'cargo');
  const artifactDirectory = join(temporaryRoot, 'artifact');
  const targetDirectory = join(temporaryRoot, 'target with whitespace');
  const errorOutput = join(temporaryRoot, 'stderr');
  const physicalTargetDirectory = join(
    temporaryRoot,
    'physical target with whitespace'
  );
  const targetAlias = join(temporaryRoot, 'target-alias');
  const physicalArtifactDirectory = join(temporaryRoot, 'physical-artifact');
  const physicalErrorOutput = join(temporaryRoot, 'physical-stderr');
  const artifactBuilder = fileURLToPath(
    new URL(
      '../../../../distribution/macos/build-xcframework.sh',
      import.meta.url
    )
  );

  try {
    mkdirSync(fakeBin);
    writeFileSync(cargo, `#!/bin/sh\ntouch "${cargoMarker}"\nexit 99\n`);
    chmodSync(cargo, 0o755);

    const result = Bun.spawnSync(
      [
        'bash',
        '-c',
        'bash "$1" 2> "$2"',
        'explorerkit-macos-path-preflight',
        artifactBuilder,
        errorOutput,
      ],
      {
        env: {
          ...process.env,
          CARGO_TARGET_DIR: targetDirectory,
          PATH: `${fakeBin}:${process.env.PATH ?? ''}`,
          EXPLORERKIT_ARTIFACT_DIR: artifactDirectory,
        },
      },
    );

    expect(result.success).toBe(false);
    expect(readFileSync(errorOutput, 'utf8')).toContain(
      'CARGO_TARGET_DIR contains unsupported whitespace'
    );
    expect(existsSync(cargoMarker)).toBe(false);
    expect(existsSync(artifactDirectory)).toBe(false);
    expect(existsSync(targetDirectory)).toBe(false);

    mkdirSync(physicalTargetDirectory);
    symlinkSync(physicalTargetDirectory, targetAlias, 'dir');
    const physicalResult = Bun.spawnSync(
      [
        'bash',
        '-c',
        'bash "$1" 2> "$2"',
        'explorerkit-macos-physical-path-preflight',
        artifactBuilder,
        physicalErrorOutput,
      ],
      {
        env: {
          ...process.env,
          CARGO_TARGET_DIR: targetAlias,
          PATH: `${fakeBin}:${process.env.PATH ?? ''}`,
          EXPLORERKIT_ARTIFACT_DIR: physicalArtifactDirectory,
        },
      },
    );

    expect(physicalResult.success).toBe(false);
    expect(readFileSync(physicalErrorOutput, 'utf8')).toContain(
      'physical CARGO_TARGET_DIR contains unsupported whitespace'
    );
    expect(existsSync(cargoMarker)).toBe(false);
    expect(existsSync(physicalArtifactDirectory)).toBe(false);
  } finally {
    rmSync(temporaryRoot, { recursive: true, force: true });
  }
});

test('macOS artifact builder exports a physical Cargo target path', () => {
  const temporaryRoot = mkdtempSync(
    join(tmpdir(), 'explorerkit-macos-physical-target-')
  );
  const fakeBin = join(temporaryRoot, 'bin');
  const cargo = join(fakeBin, 'cargo');
  const cargoTargetRecord = join(temporaryRoot, 'cargo-target');
  const physicalTargetDirectory = join(temporaryRoot, 'physical-target');
  const targetAlias = '-target-alias';
  const artifactDirectory = join(temporaryRoot, '.explorerkit-source');
  const artifactBuilder = fileURLToPath(
    new URL(
      '../../../../distribution/macos/build-xcframework.sh',
      import.meta.url
    )
  );

  try {
    mkdirSync(fakeBin);
    mkdirSync(physicalTargetDirectory);
    symlinkSync(
      physicalTargetDirectory,
      join(temporaryRoot, targetAlias),
      'dir'
    );
    writeFileSync(
      cargo,
      `#!/bin/sh
[ "$1" = metadata ] || exit 98
printf '%s\n' "$CARGO_TARGET_DIR" > "$FAKE_CARGO_TARGET_RECORD"
exit 99
`
    );
    chmodSync(cargo, 0o755);

    const result = Bun.spawnSync(['bash', artifactBuilder], {
      cwd: temporaryRoot,
      env: {
        ...process.env,
        CARGO_TARGET_DIR: targetAlias,
        FAKE_CARGO_TARGET_RECORD: cargoTargetRecord,
        PATH: `${fakeBin}:${process.env.PATH ?? ''}`,
        EXPLORERKIT_ARTIFACT_DIR: artifactDirectory,
      },
      stderr: 'pipe',
      stdout: 'pipe',
    });

    expect(result.success).toBe(false);
    expect(readFileSync(cargoTargetRecord, 'utf8').trim()).toBe(
      realpathSync(physicalTargetDirectory)
    );
  } finally {
    rmSync(temporaryRoot, { recursive: true, force: true });
  }
});

test('macOS source preparation stages one owned framework per clean source identity', () => {
  const temporaryRoot = mkdtempSync(
    join(tmpdir(), 'explorerkit-macos-source-preparer-')
  );
  const packageRoot = join(temporaryRoot, 'installed', 'react-native-explorerkit');
  const scriptsDirectory = join(packageRoot, 'scripts');
  const preparer = join(scriptsDirectory, 'prepare-macos-source.mjs');
  const sourceRoot = join(temporaryRoot, 'source');
  const sourcePackageDirectory = join(
    sourceRoot,
    'packages',
    'react-native-explorerkit'
  );
  const builderDirectory = join(sourceRoot, 'distribution', 'macos');
  const builder = join(builderDirectory, 'build-xcframework.sh');
  const builderLog = join(temporaryRoot, 'builder.log');
  const workspaceLog = join(temporaryRoot, 'workspace.log');
  const temporaryBuilds = join(temporaryRoot, 'build-temporary');
  const sourcePackagePath = join(sourcePackageDirectory, 'package.json');
  const consumerProject = join(
    sourceRoot,
    'examples',
    'react-native-macos-app',
    'macos',
    'project.pbxproj'
  );
  const ignoredBuildInput = join(
    sourceRoot,
    'crates',
    'vendor',
    'tikv-jemalloc-sys',
    'jemalloc',
    'Makefile'
  );
  const outputRoot = join(packageRoot, '.explorerkit-source');
  const stagedFramework = join(
    outputRoot,
    'current',
    'ExplorerKit.xcframework'
  );
  const stagedInfo = join(stagedFramework, 'Info.plist');
  const unknownDirectory = join(outputRoot, 'user-content');
  const unknownFile = join(unknownDirectory, 'keep.txt');

  const runGit = (...args: string[]) => {
    const result = Bun.spawnSync(['git', '-C', sourceRoot, ...args]);
    expect(result.success).toBe(true);
  };
  const commit = (message: string, ...paths: string[]) => {
    runGit('add', ...(paths.length === 0 ? ['.'] : paths));
    runGit(
      '-c',
      'user.name=ExplorerKit Test',
      '-c',
      'user.email=explorerkit@example.invalid',
      'commit',
      '-m',
      message
    );
  };
  const runPreparer = () =>
    Bun.spawnSync(['node', preparer], {
      env: {
        ...process.env,
        FAKE_BUILDER_LOG: builderLog,
        FAKE_WORKSPACE_LOG: workspaceLog,
        EXPLORERKIT_BUILD_FROM_SOURCE: '1',
        EXPLORERKIT_SOURCE_DIR: sourceRoot,
        TMPDIR: temporaryBuilds,
      },
      stderr: 'pipe',
      stdout: 'pipe',
    });
  const builderRuns = () =>
    existsSync(builderLog)
      ? readFileSync(builderLog, 'utf8').trim().split('\n').length
      : 0;

  try {
    mkdirSync(scriptsDirectory, { recursive: true });
    mkdirSync(sourcePackageDirectory, { recursive: true });
    mkdirSync(builderDirectory, { recursive: true });
    mkdirSync(dirname(consumerProject), { recursive: true });
    mkdirSync(dirname(ignoredBuildInput), { recursive: true });
    mkdirSync(
      join(
        sourceRoot,
        'crates',
        'explorerkit-host-desktop',
        'include'
      ),
      { recursive: true }
    );
    mkdirSync(temporaryBuilds);
    copyFileSync(
      fileURLToPath(
        new URL('../../scripts/prepare-macos-source.mjs', import.meta.url)
      ),
      preparer
    );
    writeFileSync(
      join(packageRoot, 'package.json'),
      JSON.stringify({ name: 'react-native-explorerkit', version: '0.1.0' })
    );
    writeFileSync(
      sourcePackagePath,
      JSON.stringify({ name: 'react-native-explorerkit', version: '0.1.0' })
    );
    writeFileSync(consumerProject, 'binary pod embed path\n');
    writeFileSync(
      join(dirname(ignoredBuildInput), '.gitignore'),
      '/Makefile\n'
    );
    writeFileSync(join(sourceRoot, 'crates', 'Cargo.toml'), '[workspace]\n');
    writeFileSync(
      join(
        sourceRoot,
        'crates',
        'explorerkit-host-desktop',
        'include',
        'explorerkit_desktop_private.h'
      ),
      '#pragma once\n'
    );
    writeFileSync(
      builder,
      `#!/bin/bash
set -euo pipefail
workspace="\${CARGO_TARGET_DIR%/cargo-target}"
test "$(cat "\${workspace}/.explorerkit-source-owner")" = "react-native-explorerkit:macos-source:v1"
test -s "\${workspace}/.explorerkit-source-pid"
printf '%s\\n' "\${workspace}" > "\${FAKE_WORKSPACE_LOG}"
printf 'run\\n' >> "\${FAKE_BUILDER_LOG}"
case "\${EXPLORERKIT_ARTIFACT_DIR}" in
  */.explorerkit-source) ;;
  *) exit 43 ;;
esac
mkdir -p "\${CARGO_TARGET_DIR}/proof"
framework="\${EXPLORERKIT_ARTIFACT_DIR}/stage/ExplorerKit.xcframework"
mkdir -p "\${framework}"
printf 'partial\\n' > "\${EXPLORERKIT_ARTIFACT_DIR}/partial"
if [ -f fail-build ]; then
  exit 42
fi
/usr/bin/plutil -create xml1 "\${framework}/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :CFBundlePackageType string XFWK' "\${framework}/Info.plist"
/usr/libexec/PlistBuddy -c "Add :SourceCommit string $(git rev-parse HEAD)" "\${framework}/Info.plist"
ln -s Info.plist "\${framework}/Current"
/usr/bin/codesign --force --sign - "\${framework}"
`
    );
    chmodSync(builder, 0o755);
    runGit('init');
    commit('initial');

    const first = runPreparer();
    expect(first.success).toBe(true);
    expect(builderRuns()).toBe(1);
    expect(readdirSync(temporaryBuilds)).toEqual([]);
    const buildWorkspace = readFileSync(workspaceLog, 'utf8').trim();
    const firstIdentity = readFileSync(stagedInfo, 'utf8').trim();
    expect(firstIdentity).not.toBe('');
    expect(lstatSync(join(stagedFramework, 'Current')).isSymbolicLink()).toBe(
      true
    );
    expect(readlinkSync(join(stagedFramework, 'Current'))).toBe('Info.plist');
    expect(
      existsSync(join(stagedFramework, '_CodeSignature', 'CodeResources'))
    ).toBe(true);

    mkdirSync(unknownDirectory);
    writeFileSync(unknownFile, 'keep\n');
    writeFileSync(join(sourceRoot, 'CONTEXT.md'), 'keep\n');
    writeFileSync(join(builderDirectory, 'unused.txt'), 'keep\n');
    writeFileSync(consumerProject, 'source pod embed path\n');
    const repeated = runPreparer();
    expect(repeated.success).toBe(true);
    expect(builderRuns()).toBe(1);

    writeFileSync(builder, `${readFileSync(builder, 'utf8')}# dirty\n`);
    const dirtyTracked = runPreparer();
    expect(dirtyTracked.success).toBe(false);
    expect(builderRuns()).toBe(1);
    runGit('checkout', '--', 'distribution/macos/build-xcframework.sh');

    const untrackedBuildInput = join(
      sourceRoot,
      'crates',
      'explorerkit-host-desktop',
      'src',
      'untracked.rs'
    );
    mkdirSync(join(sourceRoot, 'crates', 'explorerkit-host-desktop', 'src'));
    writeFileSync(untrackedBuildInput, 'build input\n');
    const dirty = runPreparer();
    expect(dirty.success).toBe(false);
    expect(builderRuns()).toBe(1);
    rmSync(untrackedBuildInput);
    expect(runPreparer().success).toBe(true);
    expect(builderRuns()).toBe(1);

    writeFileSync(ignoredBuildInput, 'ignored build input\n');
    runGit('check-ignore', 'crates/vendor/tikv-jemalloc-sys/jemalloc/Makefile');
    const dirtyIgnored = runPreparer();
    expect(dirtyIgnored.success).toBe(false);
    expect(builderRuns()).toBe(1);
    rmSync(ignoredBuildInput);

    writeFileSync(
      join(stagedFramework, '_CodeSignature', 'CodeResources'),
      'damaged\n'
    );
    mkdirSync(buildWorkspace);
    const unownedWorkspaceFile = join(buildWorkspace, 'keep');
    writeFileSync(unownedWorkspaceFile, 'keep\n');
    expect(runPreparer().success).toBe(false);
    expect(builderRuns()).toBe(1);
    expect(readFileSync(unownedWorkspaceFile, 'utf8')).toBe('keep\n');
    writeFileSync(
      join(buildWorkspace, '.explorerkit-source-owner'),
      'react-native-explorerkit:macos-source:v1\n'
    );
    const markerOnly = runPreparer();
    expect(markerOnly.success).toBe(true);
    expect(builderRuns()).toBe(2);
    expect(existsSync(unownedWorkspaceFile)).toBe(false);
    expect(readdirSync(temporaryBuilds)).toEqual([]);
    expect(readFileSync(join(sourceRoot, 'CONTEXT.md'), 'utf8')).toBe(
      'keep\n'
    );
    expect(
      Bun.spawnSync([
        '/usr/bin/codesign',
        '--verify',
        '--deep',
        '--strict',
        stagedFramework,
      ]).success
    ).toBe(true);

    mkdirSync(buildWorkspace);
    writeFileSync(
      join(buildWorkspace, '.explorerkit-source-owner'),
      'react-native-explorerkit:macos-source:v1\n'
    );
    writeFileSync(join(buildWorkspace, '.explorerkit-source-pid'), '999999\n');
    writeFileSync(join(buildWorkspace, 'partial-build'), 'stale\n');
    writeFileSync(join(sourceRoot, 'source-marker'), 'changed\n');
    commit('change source identity', 'source-marker');
    const changed = runPreparer();
    expect(changed.success).toBe(true);
    expect(builderRuns()).toBe(3);
    const changedIdentity = readFileSync(stagedInfo, 'utf8').trim();
    expect(changedIdentity).not.toBe('');
    expect(changedIdentity).not.toBe(firstIdentity);
    expect(readFileSync(unknownFile, 'utf8')).toBe('keep\n');
    expect(readdirSync(temporaryBuilds)).toEqual([]);

    writeFileSync(
      sourcePackagePath,
      JSON.stringify({ name: 'react-native-explorerkit', version: '9.9.9' })
    );
    commit(
      'mismatch source version',
      'packages/react-native-explorerkit/package.json'
    );
    const mismatched = runPreparer();
    expect(mismatched.success).toBe(false);
    expect(builderRuns()).toBe(3);
    expect(readFileSync(stagedInfo, 'utf8').trim()).toBe(changedIdentity);

    writeFileSync(
      sourcePackagePath,
      JSON.stringify({ name: 'react-native-explorerkit', version: '0.1.0' })
    );
    writeFileSync(join(sourceRoot, 'fail-build'), 'fail\n');
    commit(
      'fail changed source build',
      'packages/react-native-explorerkit/package.json',
      'fail-build'
    );
    const failed = runPreparer();
    expect(failed.success).toBe(false);
    expect(builderRuns()).toBe(4);
    expect(readFileSync(stagedInfo, 'utf8').trim()).toBe(changedIdentity);
    expect(readFileSync(unknownFile, 'utf8')).toBe('keep\n');
    expect(readdirSync(temporaryBuilds)).toEqual([]);
    expect(
      readdirSync(outputRoot).filter(
        (entry) =>
          entry.startsWith('.next-') || entry.startsWith('.previous-')
      )
    ).toEqual([]);

    rmSync(join(outputRoot, 'current', '.explorerkit-source-owner'));
    const unowned = runPreparer();
    expect(unowned.success).toBe(false);
    expect(builderRuns()).toBe(4);
    expect(readFileSync(stagedInfo, 'utf8').trim()).toBe(changedIdentity);
    expect(readFileSync(unknownFile, 'utf8')).toBe('keep\n');
  } finally {
    rmSync(temporaryRoot, { recursive: true, force: true });
  }
}, 15_000);
