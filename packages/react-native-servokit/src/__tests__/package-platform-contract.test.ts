import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { expect, test } from 'bun:test';

test('packed platform declarations include Android, iOS, and source Windows adapter', () => {
  const packageJson = JSON.parse(
    readFileSync(new URL('../../package.json', import.meta.url), 'utf8')
  ) as {
    files: string[];
    scripts: Record<string, string>;
    peerDependencies: Record<string, string>;
    peerDependenciesMeta?: Record<string, { optional?: boolean }>;
    codegenConfig: Record<string, unknown>;
  };
  const examplePackageJson = JSON.parse(
    readFileSync(new URL('../../example/package.json', import.meta.url), 'utf8')
  ) as {
    dependencies: Record<string, string>;
    devDependencies: Record<string, string>;
  };
  const reactNativeConfig = readFileSync(
    new URL('../../react-native.config.js', import.meta.url),
    'utf8'
  );
  const servoViewSource = readFileSync(
    new URL('../ServoView.tsx', import.meta.url),
    'utf8'
  );
  const androidSettings = readFileSync(
    new URL('../../example/android/settings.gradle', import.meta.url),
    'utf8'
  );
  const verifier = readFileSync(
    new URL('../../scripts/validate-packed-consumer.mjs', import.meta.url),
    'utf8'
  );
  const windowsAutolinkChecker = readFileSync(
    new URL('../../scripts/check-windows-autolink-config.mjs', import.meta.url),
    'utf8'
  );
  const prepackVerifier = fileURLToPath(
    new URL('../../scripts/verify-prepack-artifacts.mjs', import.meta.url)
  );
  const prepackVerifierSource = readFileSync(prepackVerifier, 'utf8');

  expect(packageJson.files).toContain(
    'android/libs/servokit-android-host-release.aar'
  );
  expect(packageJson.files).toContain('Servokit.podspec');
  expect(packageJson.files).toContain('ios/ServoView.mm');
  expect(packageJson.files).toContain('ios/ServoKitController.xcframework');
  expect(packageJson.files).toContain('windows');
  expect(packageJson.peerDependencies['react-native-windows']).toBe('*');
  expect(packageJson.peerDependenciesMeta?.['react-native-windows']?.optional).toBe(
    true
  );
  expect(examplePackageJson.dependencies['react-native-windows']).toBe(
    '0.85.0-preview.1'
  );
  expect(examplePackageJson.devDependencies['react-native-windows']).toBeUndefined();
  expect(packageJson.files).not.toContain('macos');
  expect(reactNativeConfig).not.toContain('ios: null');
  expect(reactNativeConfig).toContain('macos: null');
  expect(reactNativeConfig).toContain("sourceDir: 'windows'");
  expect(reactNativeConfig).toContain("solutionFile: 'ServoKit.sln'");
  expect(reactNativeConfig).toContain("projectFile: 'ServoKit\\\\ServoKit.vcxproj'");
  expect(reactNativeConfig).toContain('directDependency: true');
  expect(packageJson.codegenConfig).toHaveProperty('windows');
  expect(packageJson.codegenConfig.windows).toEqual({
    namespace: 'ServoKitCodegen',
    generators: ['componentsWindows', 'modulesWindows'],
    outputDirectory: 'windows/ServoKit/codegen',
    separateDataTypes: true,
  });
  expect(packageJson.scripts.prepack).toBe(
    'node scripts/verify-prepack-artifacts.mjs'
  );
  expect(servoViewSource).toContain(
    "Platform.OS !== 'windows'"
  );
  expect(servoViewSource).toContain(
    'currently supports Android, iOS, and Windows'
  );
  expect(servoViewSource).not.toContain("'iosCommandJson'");
  expect(androidSettings).toContain('autolinkLibrariesFromCommand()');
  expect(androidSettings).not.toContain("include ':servokit-android-host'");
  expect(androidSettings).not.toContain('projectDir = servoKitHostDir');
  expect(prepackVerifierSource).toContain(
    'android/libs/servokit-android-host-release.aar'
  );
  expect(verifier).toContain('windows/ServoKit.sln');
  expect(verifier).toContain('windows/ServoKit/ServoKit.vcxproj');
  expect(verifier).toContain('windows/ServoKit/ServoView.cpp');
  expect(verifier).toContain(
    'windows/ServoKit/codegen/react/components/ServoViewSpec/ServoView.g.h'
  );
  expect(verifier).toContain('windows/ServoKit/ReactPackageProvider.cpp');
  expect(verifier).toContain('androidAarPath');
  expect(verifier).toContain('jni/arm64-v8a/libservokit_host_android.so');
  expect(verifier).toContain('jni/x86_64/libservokit_host_android.so');
  expect(verifier).toContain('assertExactAndroidJniEntries(entries)');
  expect(verifier).not.toContain('buildRustAndroidDebug');
  expect(verifier).not.toContain('servokit_host_desktop.dll');
  expect(verifier).toContain('process.argv.includes("--ios-only")');
  expect(verifier).toContain('label: "device-arm64"');
  expect(verifier).toContain('label: "simulator-arm64"');
  expect(verifier).toContain('label: "simulator-x86_64"');
  expect(verifier).toContain('SERVOKIT_PACKED_CONSUMER_ROOT must be an absolute path');
  expect(verifier).toContain('SERVOKIT_PACKED_CONSUMER_ROOT must not already exist');
  expect(windowsAutolinkChecker).toContain('React Native Windows did not register');
  expect(windowsAutolinkChecker).toContain('ServoKit/ServoKit.vcxproj');

  const selfCheck = Bun.spawnSync(['node', prepackVerifier, '--self-check']);
  expect(selfCheck.success).toBe(true);
  expect(selfCheck.stdout.toString()).toBe('');
  expect(selfCheck.stderr.toString()).toBe('');
});
