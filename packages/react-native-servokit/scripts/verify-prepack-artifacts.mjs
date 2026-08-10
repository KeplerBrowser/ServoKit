#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const packageRoot = path.resolve(path.dirname(scriptPath), "..");

export const androidAarPath = "android/libs/servokit-android-host-release.aar";
export const requiredIosControllerFiles = [
  "ios/ServoKitController.xcframework/Info.plist",
  "ios/ServoKitController.xcframework/ios-arm64/libservokit_controller_ffi.a",
  "ios/ServoKitController.xcframework/ios-arm64/Headers/ServoKitController.h",
  "ios/ServoKitController.xcframework/ios-arm64_x86_64-simulator/libservokit_controller_ffi.a",
  "ios/ServoKitController.xcframework/ios-arm64_x86_64-simulator/Headers/ServoKitController.h",
];

export const requiredAndroidJniPayloads = [
  "jni/arm64-v8a/libc++_shared.so",
  "jni/arm64-v8a/libservokit_host_android.so",
  "jni/x86_64/libc++_shared.so",
  "jni/x86_64/libservokit_host_android.so",
];

function assertRegularFile(root, relativePath) {
  const absolutePath = path.join(root, relativePath);
  let stats;
  try {
    stats = statSync(absolutePath);
  } catch (error) {
    throw new Error(`missing required package artifact: ${relativePath}`, { cause: error });
  }
  assert(stats.isFile(), `required package artifact is not a file: ${relativePath}`);
  return absolutePath;
}

export function assertExactAndroidJniEntries(entries) {
  const jniEntries = entries.filter((entry) => entry.startsWith("jni/"));
  const payloads = jniEntries.filter((entry) => !entry.endsWith("/")).sort();
  const abis = [
    ...new Set(
      jniEntries
        .map((entry) => entry.split("/")[1])
        .filter(Boolean),
    ),
  ].sort();

  assert.deepEqual(
    payloads,
    requiredAndroidJniPayloads,
    "Android AAR must contain exactly the required JNI payloads",
  );
  assert.deepEqual(
    abis,
    ["arm64-v8a", "x86_64"],
    "Android AAR must contain exactly the supported JNI ABIs",
  );
}

export function verifyPrepackArtifacts(root = packageRoot) {
  const aar = assertRegularFile(root, androidAarPath);
  for (const relativePath of requiredIosControllerFiles) {
    assertRegularFile(root, relativePath);
  }

  const entries = execFileSync("/usr/bin/unzip", ["-Z1", aar], {
    encoding: "utf8",
  })
    .split(/\r?\n/)
    .filter(Boolean);
  assertExactAndroidJniEntries(entries);
}

function selfCheck() {
  assert.doesNotThrow(() => assertExactAndroidJniEntries(requiredAndroidJniPayloads));
  assert.throws(() => assertExactAndroidJniEntries(requiredAndroidJniPayloads.slice(1)));
  assert.throws(() =>
    assertExactAndroidJniEntries([
      ...requiredAndroidJniPayloads,
      "jni/armeabi-v7a/libservokit_host_android.so",
    ]),
  );
  assert.throws(() =>
    assertExactAndroidJniEntries([
      ...requiredAndroidJniPayloads,
      "jni/arm64-v8a/libunexpected.so",
    ]),
  );
  assert.throws(() =>
    assertExactAndroidJniEntries([
      ...requiredAndroidJniPayloads,
      "jni/armeabi-v7a/",
    ]),
  );
}

if (process.argv[1] && path.resolve(process.argv[1]) === scriptPath) {
  if (process.argv.includes("--self-check")) {
    selfCheck();
  } else {
    verifyPrepackArtifacts();
  }
}
