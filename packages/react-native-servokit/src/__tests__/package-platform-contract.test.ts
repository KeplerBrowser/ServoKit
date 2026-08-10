import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { expect, test } from 'bun:test';

test('packed platform declarations include Android and iOS only', () => {
  const packageJson = JSON.parse(
    readFileSync(new URL('../../package.json', import.meta.url), 'utf8')
  ) as {
    files: string[];
    scripts: Record<string, string>;
    codegenConfig: Record<string, unknown>;
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
  expect(packageJson.files).not.toContain('macos');
  expect(reactNativeConfig).not.toContain('ios: null');
  expect(reactNativeConfig).toContain('macos: null');
  expect(reactNativeConfig).toContain('windows: null');
  expect(packageJson.codegenConfig).not.toHaveProperty('windows');
  expect(packageJson.scripts.prepack).toBe(
    'node scripts/verify-prepack-artifacts.mjs'
  );
  expect(servoViewSource).toContain(
    "if (Platform.OS !== 'android' && Platform.OS !== 'ios') {"
  );
  expect(servoViewSource).toContain('currently supports Android and iOS');
  expect(servoViewSource).not.toContain("'iosCommandJson'");
  expect(androidSettings).toContain('autolinkLibrariesFromCommand()');
  expect(androidSettings).not.toContain("include ':servokit-android-host'");
  expect(androidSettings).not.toContain('projectDir = servoKitHostDir');
  expect(prepackVerifierSource).toContain(
    'android/libs/servokit-android-host-release.aar'
  );
  expect(verifier).toContain('androidAarPath');
  expect(verifier).toContain('jni/arm64-v8a/libservokit_host_android.so');
  expect(verifier).toContain('jni/x86_64/libservokit_host_android.so');
  expect(verifier).toContain('assertExactAndroidJniEntries(entries)');
  expect(verifier).not.toContain('buildRustAndroidDebug');
  expect(verifier).toContain('process.argv.includes("--ios-only")');
  expect(verifier).toContain('label: "device-arm64"');
  expect(verifier).toContain('label: "simulator-arm64"');
  expect(verifier).toContain('label: "simulator-x86_64"');
  expect(verifier).toContain('SERVOKIT_PACKED_CONSUMER_ROOT must be an absolute path');
  expect(verifier).toContain('SERVOKIT_PACKED_CONSUMER_ROOT must not already exist');

  const selfCheck = Bun.spawnSync(['node', prepackVerifier, '--self-check']);
  expect(selfCheck.success).toBe(true);
  expect(selfCheck.stdout.toString()).toBe('');
  expect(selfCheck.stderr.toString()).toBe('');
});
