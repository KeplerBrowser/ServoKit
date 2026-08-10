#!/usr/bin/env node

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { once } from "node:events";
import { constants, createReadStream } from "node:fs";
import {
  access,
  cp,
  lstat,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  realpath,
  rename,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

import {
  androidAarPath,
  assertExactAndroidJniEntries,
  requiredIosControllerFiles,
} from "./verify-prepack-artifacts.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const packageRoot = fileURLToPath(new URL("..", import.meta.url));
const repoRoot = path.resolve(packageRoot, "../..");
const exampleRoot = path.join(packageRoot, "example");
const canonicalFixtureRoot = path.join(repoRoot, "examples", "fixtures");
const negativeControlsOnly = process.argv.includes("--negative-controls-only");
const iosOnly = process.argv.includes("--ios-only");
const lifecycleSelfCheckOnly = process.argv.includes("--lifecycle-self-check-only");
const signalLifecycleChild = process.argv.includes("--signal-lifecycle-child");
const generatedPackagePaths = ["lib"];
const androidFiles = [
  "android/build.gradle",
  "android/src/main/AndroidManifest.xml",
  androidAarPath,
];
const iosControllerFiles = requiredIosControllerFiles;
const iosFiles = ["Servokit.podspec", "ios/ServoView.mm", ...iosControllerFiles];
const approvedAppleBinaryFiles = new Set(iosControllerFiles);
const requiredPackageFiles = [
  "package.json",
  "lib/module/index.js",
  "lib/typescript/src/index.d.ts",
  "src/ServoViewNativeComponent.ts",
  ...androidFiles,
  ...iosFiles,
];
const negativeControlFiles = [
  "android/build.gradle",
  "android/libs/servokit-android-host-release.aar",
  ...iosFiles,
];
const forbiddenPackageFiles = new Set([
  "android/CMakeLists.txt",
  "android/consumer-rules.pro",
  "android/cpp-adapter.cpp",
  "android/src/main/java/org/servo/servokit/reactnative/ServokitModule.kt",
  "ios/Servokit.h",
  "ios/Servokit.mm",
  "src/ServokitBindings.ts",
  "ubrn.config.yaml",
]);
const forbiddenPackagePrefixes = [
  "android/servokit-android-host/",
  "cpp/",
  "crates/",
  "lib/module/generated/bindings/",
  "lib/typescript/src/generated/bindings/",
  "src/generated/bindings/",
];
const appleBinaryControls = [
  "ios/negative-control.a",
  "ios/negative-control.dylib",
  "ios/Negative.framework/Negative",
  "ios/Negative.xcframework/ios-arm64/Negative.framework/Negative",
  "ios/ServoKitController.xcframework/ios-arm64/unapproved.a",
  "ios/ServoKitController.xcframework/ios-arm64/unapproved.dylib",
  "ios/ServoKitController.xcframework/ios-arm64/Unapproved.framework/Unapproved",
];
const systemPathEntries = ["/usr/bin", "/bin", "/usr/sbin", "/sbin"];
const signalExitCodes = { SIGINT: 130, SIGTERM: 143 };
const controllerSymbols = [
  "servokit_controller_create",
  "servokit_controller_dispatch",
  "servokit_controller_result_free",
  "servokit_controller_destroy",
];
const iosMatrices = [
  {
    label: "device-arm64",
    architecture: "arm64",
    sdk: "iphoneos",
    destination: "generic/platform=iOS",
    productDirectory: "Release-iphoneos",
    slice: "ios-arm64",
    onlyActiveArchitecture: "YES",
  },
  {
    label: "simulator-arm64",
    architecture: "arm64",
    sdk: "iphonesimulator",
    destination: "generic/platform=iOS Simulator",
    productDirectory: "Release-iphonesimulator",
    slice: "ios-arm64_x86_64-simulator",
    onlyActiveArchitecture: "YES",
  },
  {
    label: "simulator-x86_64",
    architecture: "x86_64",
    sdk: "iphonesimulator",
    destination: "generic/platform=iOS Simulator",
    productDirectory: "Release-iphonesimulator",
    slice: "ios-arm64_x86_64-simulator",
    onlyActiveArchitecture: "NO",
  },
];
let activeChild;
let receivedSignal;

function signalChild(child, signal) {
  if (!child) return;
  try {
    process.kill(process.platform === "win32" ? child.pid : -child.pid, signal);
  } catch (error) {
    if (error.code !== "ESRCH") console.error(`failed to forward ${signal}: ${error.message}`);
  }
}

for (const signal of Object.keys(signalExitCodes)) {
  process.on(signal, () => {
    receivedSignal ??= signal;
    const child = activeChild;
    signalChild(child, signal);
    setTimeout(() => activeChild === child && signalChild(child, "SIGKILL"), 5_000).unref();
  });
}
const templateExcludedNames = new Set([
  ".bundle",
  ".cxx",
  ".git",
  ".gradle",
  ".kotlin",
  "DerivedData",
  "Pods",
  "build",
  "node_modules",
  "vendor",
]);

class PackageFileError extends Error {
  constructor(kind, filePath, options) {
    super(`${kind} required package file: ${filePath}`, options);
    this.name = "PackageFileError";
    this.kind = kind;
    this.path = filePath;
  }
}

class AppleBinaryError extends Error {
  constructor(paths) {
    super(`packed tarball contains unexpected Apple native binaries:\n${paths.join("\n")}`);
    this.name = "AppleBinaryError";
    this.paths = paths;
  }
}

function isInside(root, target) {
  const relative = path.relative(root, target);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function isConfigurationFile(file) {
  return /^(?:CMakeLists\.txt|Podfile|(?:babel|metro|react-native)\.config\.js|package\.json)$|\.(?:gradle|podspec|properties|toml)$/.test(
    path.basename(file),
  );
}

async function exists(file) {
  try {
    await lstat(file);
    return true;
  } catch (error) {
    if (error.code === "ENOENT") {
      return false;
    }
    throw error;
  }
}

async function sha256(file) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest("hex");
}

async function findFiles(root, name) {
  if (!(await exists(root))) return [];
  return (await readdir(root, { recursive: true, withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name === name)
    .map((entry) => path.join(entry.parentPath, entry.name));
}

async function assertRegularFile(root, relativePath) {
  const file = path.join(root, relativePath);
  let stats;
  let ioError;

  try {
    stats = await lstat(file);
  } catch (error) {
    if (error.code === "ENOENT") {
      throw new PackageFileError("missing", relativePath);
    }
    ioError = error;
  }

  if (stats?.isSymbolicLink()) {
    throw new PackageFileError("symlink", relativePath);
  }
  if (stats && !stats.isFile()) {
    throw new PackageFileError("non-file", relativePath);
  }
  if (ioError) {
    throw new PackageFileError("unreadable", relativePath, { cause: ioError });
  }

  return file;
}

async function runChecked(command, args, options = {}) {
  if (receivedSignal) throw new Error(`refusing to start ${command} after ${receivedSignal}`);
  console.log(`$ ${[command, ...args].join(" ")}`);
  const capture = options.capture ?? false;
  const child = spawn(command, args, {
    cwd: options.cwd ?? repoRoot,
    detached: process.platform !== "win32",
    env: options.env ?? process.env,
    stdio: capture ? ["ignore", "pipe", "pipe"] : "inherit",
  });
  let output = "";
  activeChild = child;
  child.stdout?.setEncoding("utf8").on("data", (chunk) => (output += chunk));
  child.stderr?.setEncoding("utf8").on("data", (chunk) => (output += chunk));

  let status;
  let signal;
  try {
    [status, signal] = await once(child, "close");
  } catch (error) {
    throw new Error(`${command} failed to start: ${error.message}`, { cause: error });
  } finally {
    if (activeChild === child) activeChild = undefined;
  }
  if (capture && options.echo) process.stdout.write(output);
  if (signal) {
    throw new Error(`${command} terminated by signal ${signal}${capture ? `\n${output}` : ""}`);
  }
  assert.equal(status, 0, `${command} exited with ${status}${capture ? `\n${output}` : " (output above)"}`);
  return capture ? output : "";
}

function parsePackJson(stdout) {
  let parsed;
  try {
    parsed = JSON.parse(stdout);
  } catch (error) {
    throw new Error("npm pack did not return parseable JSON", { cause: error });
  }

  assert(Array.isArray(parsed) && parsed.length === 1, "npm pack did not return one package result");
  return parsed[0];
}

function isUnexpectedAppleBinary(file) {
  const parts = file.toLowerCase().split("/");
  return (
    parts.some((part) => part.endsWith(".framework") || part.endsWith(".xcframework")) ||
    parts.at(-1)?.endsWith(".a") ||
    parts.at(-1)?.endsWith(".dylib")
  );
}

function assertPackShape(packResult) {
  assert(Array.isArray(packResult.files), "npm pack result did not include a file list");
  const packedFiles = new Set(packResult.files.map((file) => file.path));
  const missing = requiredPackageFiles.filter((file) => !packedFiles.has(file));
  const forbidden = packResult.files
    .map((file) => file.path)
    .filter(
      (file) =>
        forbiddenPackageFiles.has(file) ||
        forbiddenPackagePrefixes.some((prefix) => file.startsWith(prefix)) ||
        file.includes("ServokitBindings") ||
        file.endsWith("libservokit_bindings.so"),
    );
  const appleBinaries = packResult.files
    .map((file) => file.path)
    .filter((file) => isUnexpectedAppleBinary(file) && !approvedAppleBinaryFiles.has(file));

  if (missing.length > 0) {
    throw new Error(`packed tarball is missing required files:\n${missing.join("\n")}`);
  }
  if (forbidden.length > 0) {
    throw new Error(`packed tarball contains removed UBRN files:\n${forbidden.join("\n")}`);
  }
  if (appleBinaries.length > 0) {
    throw new AppleBinaryError(appleBinaries);
  }
}

function runAppleBinaryControls(packResult) {
  for (const controlPath of appleBinaryControls) {
    let rejection;
    try {
      assertPackShape({
        ...packResult,
        files: [...packResult.files, { path: controlPath }],
      });
    } catch (error) {
      rejection = error;
    }

    assert(
      rejection instanceof AppleBinaryError && rejection.paths.includes(controlPath),
      `Apple binary control was not rejected: ${controlPath}`,
    );
    console.log(`Apple binary control rejected ${controlPath}`);
  }
}

async function assertPackageShape(installedPackage) {
  for (const file of requiredPackageFiles) {
    await assertRegularFile(installedPackage, file);
  }

  const manifest = JSON.parse(
    await readFile(path.join(installedPackage, "package.json"), "utf8"),
  );
  assert(
    !manifest.dependencies?.["uniffi-bindgen-react-native"],
    "packed package retained the UBRN runtime dependency",
  );
  assert(
    !Object.keys(manifest.scripts ?? {}).some((name) => name.startsWith("ubrn:")),
    "packed package retained UBRN scripts",
  );
  for (const lifecycle of ["preinstall", "install", "postinstall"]) {
    assert(!manifest.scripts?.[lifecycle], `packed package retained ${lifecycle} lifecycle script`);
  }
  assert.equal(manifest.codegenConfig?.type, "all", "packed package changed Codegen type");
  assert.equal(
    manifest.codegenConfig?.windows,
    undefined,
    "packed package advertises unsupported Windows settings",
  );
}

async function assertComponentOnlyCodegen(installedPackage) {
  const codegenRoot = path.join(installedPackage, "android/build/generated/source/codegen");
  const schema = JSON.parse(await readFile(path.join(codegenRoot, "schema.json"), "utf8"));
  assert.deepEqual(
    Object.entries(schema.modules ?? {}).map(([name, module]) => ({ name, type: module.type })),
    [{ name: "ServoView", type: "Component" }],
    "React Native Codegen discovered inputs other than the ServoView component",
  );

  for (const file of [
    "java/com/facebook/react/viewmanagers/ServoViewManagerDelegate.java",
    "java/com/facebook/react/viewmanagers/ServoViewManagerInterface.java",
    "jni/react/renderer/components/ServoViewSpec/ComponentDescriptors.h",
  ]) {
    await assertRegularFile(codegenRoot, file);
  }

  const generatedFiles = (await readdir(codegenRoot, { recursive: true, withFileTypes: true }))
    .filter((entry) => entry.isFile())
    .map((entry) => path.relative(codegenRoot, path.join(entry.parentPath, entry.name)));
  const nativeModuleArtifacts = generatedFiles.filter(
    (file) =>
      /(?:^|\/)Native[^/]*Spec\.java$/i.test(file) ||
      /(?:ServokitBindings|ServokitModule)/i.test(file),
  );
  assert.deepEqual(
    nativeModuleArtifacts,
    [],
    `React Native Codegen emitted native-module artifacts:\n${nativeModuleArtifacts.join("\n")}`,
  );

  // `type: all` emits an empty provider scaffold even when the schema has no NativeModule.
  const moduleProvider = await readFile(
    path.join(codegenRoot, "jni/ServoViewSpec-generated.cpp"),
    "utf8",
  );
  assert.match(
    moduleProvider,
    /ServoViewSpec_ModuleProvider[^{}]*\{\s*return nullptr;\s*\}/s,
    "React Native Codegen emitted a non-empty TurboModule provider",
  );
  console.log("React Native Codegen discovered only ServoView component inputs");
}

async function inspectHermeticText(file, boundary, canonicalBoundary, canonicalRepoRoot) {
  const contents = await readFile(file);
  if (contents.includes(0)) {
    return;
  }

  const text = contents.toString("utf8");
  const label = path.relative(boundary, file);
  for (const checkoutPath of new Set([repoRoot, canonicalRepoRoot])) {
    assert(!text.includes(checkoutPath), `${label} references the ServoKit checkout`);
  }
  assert(!/\bworkspace:/.test(text), `${label} contains a workspace reference`);

  const tokens = [];
  const unquoted = text.replace(/(["'`])([^"'`\r\n]+)\1/g, (_, __, token) => {
    tokens.push([token, true]);
    return " ";
  });
  tokens.push(...unquoted.split(/[\s,;()[\]]+/).map((token) => [token, false]));

  for (const [rawToken, quoted] of tokens) {
    if (!quoted && (!/^[A-Za-z0-9@${}_.~:=/\\-]+$/.test(rawToken) || !/[A-Za-z0-9_.]/.test(rawToken))) {
      continue;
    }
    const token = rawToken.replace(/^[^=/\\]+=/, "").replace(/^file:/, "");
    const nativeAbsolute = path.isAbsolute(token);
    const windowsAbsolute = /^(?:[A-Za-z]:[/\\]|\\\\[^/\\]+[/\\][^/\\]+)/.test(token);
    const hasParentSegment = token.split(/[/\\]+/).includes("..");
    if (!nativeAbsolute && !windowsAbsolute && !hasParentSegment) {
      continue;
    }

    assert(
      !windowsAbsolute || nativeAbsolute,
      `${label} references a path outside the consumer: ${rawToken}`,
    );
    const target = nativeAbsolute
      ? path.resolve(token)
      : path.resolve(path.dirname(file), token.replaceAll("\\", path.sep));
    assert(isInside(boundary, target), `${label} references a path outside the consumer: ${rawToken}`);
    try {
      assert(
        isInside(canonicalBoundary, await realpath(target)),
        `${label} resolves a path outside the consumer: ${rawToken}`,
      );
    } catch (error) {
      if (error.code !== "ENOENT") {
        throw error;
      }
    }
  }
}

async function assertHermeticTree(root, boundary = root) {
  const canonicalBoundary = await realpath(boundary);
  const canonicalRepoRoot = await realpath(repoRoot);

  for (const entry of await readdir(root, { recursive: true, withFileTypes: true })) {
    const target = path.join(entry.parentPath, entry.name);
    const label = path.relative(boundary, target);
    assert(!entry.isSymbolicLink(), `staged source contains symlink: ${label}`);

    const canonicalTarget = await realpath(target);
    assert(
      isInside(canonicalBoundary, canonicalTarget),
      `staged source resolves outside the consumer: ${label} -> ${canonicalTarget}`,
    );

    if (entry.isFile()) {
      if (isConfigurationFile(target)) {
        await inspectHermeticText(target, boundary, canonicalBoundary, canonicalRepoRoot);
      }
    } else if (!entry.isDirectory()) {
      throw new Error(`staged source contains unsupported filesystem entry: ${label}`);
    }
  }
}

async function backupPackageState(state, controls) {
  for (const relativePath of state.generatedPaths) {
    const source = path.join(state.packageDir, relativePath);
    if (!(await exists(source))) {
      continue;
    }

    await controls.backup?.(relativePath);
    const backup = path.join(state.backupRoot, relativePath);
    await mkdir(path.dirname(backup), { recursive: true });
    await cp(source, backup, {
      recursive: true,
      dereference: false,
      preserveTimestamps: true,
      verbatimSymlinks: true,
    });
    state.preserved.add(relativePath);
  }
  state.backupComplete = true;
}

async function restorePackageState(state, controls) {
  const failures = [];

  for (const relativePath of state.generatedPaths) {
    try {
      await controls.restore?.(relativePath);
      const source = path.join(state.packageDir, relativePath);
      await rm(source, { recursive: true, force: true });

      if (state.preserved.has(relativePath)) {
        const backup = path.join(state.backupRoot, relativePath);
        await mkdir(path.dirname(source), { recursive: true });
        await cp(backup, source, {
          recursive: true,
          dereference: false,
          preserveTimestamps: true,
          verbatimSymlinks: true,
        });
        await rm(backup, { recursive: true, force: true });
      }
    } catch (error) {
      failures.push(new Error(`failed to restore ${relativePath}`, { cause: error }));
    }
  }

  if (failures.length > 0) {
    throw failures.length === 1
      ? failures[0]
      : new AggregateError(failures, "multiple package paths failed to restore");
  }
}

async function runProtectedValidation(
  tempRoot,
  packageDir,
  generatedPaths,
  work,
  controls = {},
) {
  const state = {
    backupComplete: false,
    backupRoot: path.join(tempRoot, "package-state"),
    generatedPaths,
    packageDir,
    preserved: new Set(),
  };
  let primaryFailure;
  let restorationFailure;
  let removalFailure;
  let tempRemoved = false;

  try {
    await backupPackageState(state, controls);
    await work();
  } catch (error) {
    primaryFailure = error;
  }

  if (state.backupComplete) {
    try {
      await restorePackageState(state, controls);
    } catch (error) {
      restorationFailure = new Error(
        `package state restoration failed; recovery data retained at ${tempRoot}`,
        { cause: error },
      );
    }
  }

  if (!restorationFailure && !controls.retainTemp) {
    try {
      await controls.removeTemp?.();
      await rm(tempRoot, { recursive: true, force: true });
      tempRemoved = true;
    } catch (error) {
      removalFailure = new Error(`temporary validation data retained at ${tempRoot}`, {
        cause: error,
      });
    }
  }

  if (receivedSignal) {
    const restored = !restorationFailure && !removalFailure;
    primaryFailure = new Error(
      `parent received ${receivedSignal}${
        restored
          ? controls.retainTemp
            ? `; package state restored and temporary data retained at ${tempRoot}`
            : tempRemoved
              ? "; package state restored and temporary data removed"
              : "; package state restored"
          : ""
      }`,
      { cause: primaryFailure },
    );
  }

  if (controls.retainTemp && !restorationFailure) {
    console.log(`retained exact consumer at ${tempRoot}`);
  }

  const failures = [primaryFailure, restorationFailure, removalFailure].filter(Boolean);
  if (failures.length === 1) {
    throw failures[0];
  }
  if (failures.length > 1) {
    throw new AggregateError(failures, "packed consumer validation and cleanup failed");
  }
}

async function captureFailure(work) {
  try {
    await work();
  } catch (error) {
    return error;
  }
  throw new Error("expected operation to fail");
}

async function withLifecycleFixture(label, check) {
  const fixtureRoot = await mkdtemp(path.join(tmpdir(), `servokit-lifecycle-${label}-`));
  const packageDir = path.join(fixtureRoot, "package");
  const generatedDir = path.join(packageDir, "generated");
  const tempRoot = await mkdtemp(path.join(fixtureRoot, "run-"));
  await mkdir(generatedDir, { recursive: true });
  await writeFile(path.join(generatedDir, "state.txt"), "original\n");

  try {
    await check({ generatedDir, packageDir, tempRoot });
  } finally {
    await rm(fixtureRoot, { recursive: true, force: true });
  }
}

async function replaceLifecycleFixture(generatedDir) {
  await rm(generatedDir, { recursive: true, force: true });
  await mkdir(generatedDir, { recursive: true });
  await writeFile(path.join(generatedDir, "state.txt"), "generated\n");
}

async function selfCheckLifecycle() {
  await withLifecycleFixture("normal", async ({ generatedDir, packageDir, tempRoot }) => {
    await runProtectedValidation(tempRoot, packageDir, ["generated"], async () => {
      await replaceLifecycleFixture(generatedDir);
    });
    assert(!(await exists(tempRoot)), "successful validation did not remove its temporary root");
    assert.equal(await readFile(path.join(generatedDir, "state.txt"), "utf8"), "original\n");
  });

  await withLifecycleFixture("backup", async ({ packageDir, tempRoot }) => {
    const expected = new Error("injected backup failure");
    const failure = await captureFailure(() =>
      runProtectedValidation(tempRoot, packageDir, ["generated"], async () => {
        throw new Error("validation must not run after backup failure");
      }, {
        backup() {
          throw expected;
        },
      }),
    );
    assert(failure === expected, "backup failure was not preserved");
    assert(!(await exists(tempRoot)), "backup failure did not remove its temporary root");
  });

  await withLifecycleFixture("validation", async ({ generatedDir, packageDir, tempRoot }) => {
    const expected = new Error("injected validation failure");
    const failure = await captureFailure(() =>
      runProtectedValidation(tempRoot, packageDir, ["generated"], async () => {
        await replaceLifecycleFixture(generatedDir);
        throw expected;
      }),
    );
    assert(failure === expected, "ordinary validation failure was not preserved");
    assert(!(await exists(tempRoot)), "ordinary validation failure did not remove its temporary root");
    assert(
      (await readFile(path.join(generatedDir, "state.txt"), "utf8")) === "original\n",
      "ordinary validation failure did not restore package state",
    );
  });

  await withLifecycleFixture("restore", async ({ generatedDir, packageDir, tempRoot }) => {
    const primary = new Error("injected validation failure");
    const restore = new Error("injected restoration failure");
    const failure = await captureFailure(() =>
      runProtectedValidation(tempRoot, packageDir, ["generated"], async () => {
        await replaceLifecycleFixture(generatedDir);
        throw primary;
      }, {
        restore() {
          throw restore;
        },
      }),
    );
    assert(failure instanceof AggregateError, "restoration failure was not aggregated");
    assert(failure.errors[0] === primary, "restoration failure replaced the primary failure");
    assert(failure.errors[1].cause.cause === restore, "restoration failure cause was not retained");
    assert(failure.errors[1].message.includes(tempRoot), "restoration failure omitted recovery path");
    assert(await exists(tempRoot), "restoration failure removed recovery data");
    assert(
      await exists(path.join(tempRoot, "package-state", "generated", "state.txt")),
      "restoration failure removed the package backup",
    );
  });

  await withLifecycleFixture("removal", async ({ generatedDir, packageDir, tempRoot }) => {
    const primary = new Error("injected validation failure");
    const removal = new Error("injected temporary removal failure");
    const failure = await captureFailure(() =>
      runProtectedValidation(tempRoot, packageDir, ["generated"], async () => {
        await replaceLifecycleFixture(generatedDir);
        throw primary;
      }, {
        removeTemp() {
          throw removal;
        },
      }),
    );
    assert(failure instanceof AggregateError, "temporary removal failure was not aggregated");
    assert(failure.errors[0] === primary, "temporary removal failure replaced the primary failure");
    assert(failure.errors[1].cause === removal, "temporary removal failure cause was not retained");
    assert(failure.errors[1].message.includes(tempRoot), "temporary removal failure omitted retained path");
    assert(await exists(tempRoot), "temporary removal failure did not retain its temporary root");
    assert(
      (await readFile(path.join(generatedDir, "state.txt"), "utf8")) === "original\n",
      "temporary removal failure did not restore package state",
    );
  });

  await withLifecycleFixture("retained", async ({ generatedDir, packageDir, tempRoot }) => {
    await runProtectedValidation(
      tempRoot,
      packageDir,
      ["generated"],
      async () => replaceLifecycleFixture(generatedDir),
      { retainTemp: true },
    );
    assert(await exists(tempRoot), "retained validation removed its temporary root");
    assert.equal(await readFile(path.join(generatedDir, "state.txt"), "utf8"), "original\n");
  });

  console.log("cleanup lifecycle self-checks passed");
}

async function runSignalLifecycleChildMode() {
  const [packageDir, tempRoot] = process.argv.slice(-2);
  assert(packageDir && tempRoot, "signal lifecycle child paths were not provided");

  await runProtectedValidation(tempRoot, packageDir, ["generated"], async () => {
    await replaceLifecycleFixture(path.join(packageDir, "generated"));
    const runningChild = runChecked(process.execPath, ["-e", "setInterval(() => {}, 1_000)"], { capture: true });
    process.send?.("ready");
    await runningChild;
  });
}

async function runSignalLifecycleProbe(signal, packageDir, tempRoot) {
  const child = spawn(process.execPath, [scriptPath, "--signal-lifecycle-child", packageDir, tempRoot], {
    killSignal: "SIGKILL", stdio: ["ignore", "pipe", "pipe", "ipc"], timeout: 10_000,
  });
  let output = "";
  for (const stream of [child.stdout, child.stderr]) stream.on("data", (chunk) => (output += chunk));
  child.once("message", () => child.kill(signal));
  const [code] = await once(child, "close");
  return { code, output };
}

async function selfCheckSignals() {
  await Promise.all(
    Object.entries(signalExitCodes).map(([signal, exitCode]) =>
      withLifecycleFixture(signal.toLowerCase(), async ({ generatedDir, packageDir, tempRoot }) => {
        const { code, output } = await runSignalLifecycleProbe(signal, packageDir, tempRoot);
        assert.equal(code, exitCode, `${signal} used the wrong exit code`);
        assert.match(
          output,
          new RegExp(`parent received ${signal}; package state restored and temporary data removed`),
        );
        assert.equal(await exists(tempRoot), false, `${signal} retained temporary data`);
        assert.equal(await readFile(path.join(generatedDir, "state.txt"), "utf8"), "original\n");
      }),
    ),
  );
  console.log("signal lifecycle self-checks passed");
}

async function selfCheckHermeticPaths() {
  await withLifecycleFixture("paths", async ({ packageDir }) => {
    const fixtureFile = path.join(packageDir, "paths.json");
    const canonicalBoundary = await realpath(packageDir);
    const canonicalRepoRoot = await realpath(repoRoot);
    const scan = async (text) => {
      await writeFile(fixtureFile, text);
      return inspectHermeticText(fixtureFile, packageDir, canonicalBoundary, canonicalRepoRoot);
    };
    await scan("path=safe/../inside\n\\\n");

    for (const staleConfig of [
      `org.gradle.java.home=${path.join(path.dirname(packageDir), "missing-outside")}`,
      `path=${path.join(path.dirname(packageDir), "missing=outside")}`,
      `path="${path.join(path.dirname(packageDir), "missing outside")}"`,
      "dependency=workspace:*", repoRoot, canonicalRepoRoot,
      "path=C:\\missing-outside", "path=\\\\server\\share",
      "path=safe/../../../missing-outside",
    ]) {
      await captureFailure(() => scan(staleConfig));
    }
  });

  console.log("configuration path controls passed");
}

async function packPackage(tempRoot) {
  const output = await runChecked(
    "npm",
    [
      "pack",
      "--json",
      "--silent",
      "--foreground-scripts=false",
      "--pack-destination",
      tempRoot,
    ],
    {
      capture: true,
      cwd: packageRoot,
      env: { ...process.env, npm_config_cache: path.join(tempRoot, "npm-cache") },
    },
  );
  const packResult = parsePackJson(output);
  assertPackShape(packResult);
  runAppleBinaryControls(packResult);
  assert(path.basename(packResult.filename) === packResult.filename, "npm pack returned an unsafe filename");

  const tarball = await assertRegularFile(tempRoot, packResult.filename);
  console.log(
    `packed ${packResult.files.length} files into ${path.basename(tarball)} sha256=${await sha256(tarball)}`,
  );
  return tarball;
}

async function assertExampleConfiguration(root, expectedPackageRoot, workspaceMode) {
  const require = createRequire(path.join(root, "package.json"));
  const babelConfig = require(path.join(root, "babel.config.js"));
  const metroConfig = require(path.join(root, "metro.config.js"));
  const reactNativeConfig = require(path.join(root, "react-native.config.js"));
  const expectedPackagePath = await realpath(expectedPackageRoot);
  assert.equal(
    await realpath(reactNativeConfig.dependencies["react-native-servokit"].root),
    expectedPackagePath,
    "React Native config selected the wrong package",
  );
  assert(
    babelConfig.overrides?.some(
      (override) => path.resolve(override.include) === path.join(expectedPackagePath, "src"),
    ),
    "Babel config selected the wrong package source",
  );

  const watchFolders = metroConfig.watchFolders ?? [];
  const canonicalRoot = await realpath(root);
  const canonicalFolders = await Promise.all(watchFolders.map((folder) => realpath(folder)));
  assert(
    workspaceMode
      ? watchFolders.some((folder) => path.resolve(folder) === repoRoot)
      : watchFolders.every(
          (folder, index) =>
            isInside(root, path.resolve(folder)) && isInside(canonicalRoot, canonicalFolders[index]),
        ),
    `${workspaceMode ? "workspace" : "installed"} Metro config selected the wrong root`,
  );

  console.log(`${workspaceMode ? "workspace" : "installed"} RN/Metro/Babel config passed`);
}

async function stageConsumer(tarball, consumerRoot) {
  await cp(exampleRoot, consumerRoot, {
    recursive: true,
    filter(source) {
      const relative = path.relative(exampleRoot, source);
      if (relative === "") {
        return true;
      }
      if (
        relative === path.join("android", "local.properties") ||
        relative === path.join("ios", ".xcode.env.local") ||
        relative === path.join("ios", "Podfile.lock")
      ) {
        return false;
      }
      return !relative.split(path.sep).some((part) => templateExcludedNames.has(part));
    },
  });

  const fixtureRoot = path.join(consumerRoot, "fixtures");
  await cp(canonicalFixtureRoot, fixtureRoot, { recursive: true });
  await assertCanonicalFixtures(fixtureRoot);
  assert(
    isInside(await realpath(consumerRoot), await realpath(fixtureRoot)),
    "canonical fixtures were staged outside the temporary consumer",
  );

  const stagedTarball = path.join(consumerRoot, path.basename(tarball));
  await cp(tarball, stagedTarball);
  const packageJsonPath = path.join(consumerRoot, "package.json");
  const packageJson = JSON.parse(await readFile(packageJsonPath, "utf8"));
  assert(packageJson.dependencies?.["react-native-servokit"], "example dependency is missing");
  packageJson.dependencies["react-native-servokit"] = `file:./${path.basename(stagedTarball)}`;
  await writeFile(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);

  await assertHermeticTree(consumerRoot);
  return stagedTarball;
}

async function installConsumer(consumerRoot, tempRoot, stagedTarball) {
  await runChecked("bun", ["install", "--ignore-scripts", "--no-progress"], {
    cwd: consumerRoot,
    env: {
      ...process.env,
      BUN_INSTALL_CACHE_DIR: path.join(tempRoot, "bun-cache"),
      npm_config_cache: path.join(tempRoot, "npm-install-cache"),
    },
  });

  const packageJson = JSON.parse(await readFile(path.join(consumerRoot, "package.json"), "utf8"));
  assert(
    packageJson.dependencies["react-native-servokit"] === `file:./${path.basename(stagedTarball)}`,
    "consumer did not retain the exact staged tarball dependency",
  );

  const installedPackage = path.join(consumerRoot, "node_modules", "react-native-servokit");
  const installedStats = await lstat(installedPackage);
  const installedRealPath = await realpath(installedPackage);
  const consumerRealPath = await realpath(consumerRoot);
  const repoRealPath = await realpath(repoRoot);

  assert(!installedStats.isSymbolicLink(), "react-native-servokit was installed as a symlink");
  assert(installedStats.isDirectory(), "react-native-servokit was not installed as a directory");
  assert(
    isInside(consumerRealPath, installedRealPath),
    `react-native-servokit resolved outside the isolated consumer: ${installedRealPath}`,
  );
  assert(
    !isInside(repoRealPath, installedRealPath),
    "react-native-servokit resolved inside the ServoKit checkout",
  );
  await assertHermeticTree(installedPackage, consumerRoot);
  await assertPackageShape(installedPackage);
  await assertExampleConfiguration(consumerRoot, installedPackage, false);
  return installedPackage;
}

async function runMissingFileControls(installedPackage) {
  for (const relativePath of negativeControlFiles) {
    const requiredFile = path.join(installedPackage, relativePath);
    const hiddenFile = `${requiredFile}.negative-control`;
    await rename(requiredFile, hiddenFile);
    try {
      const rejection = await captureFailure(() => assertPackageShape(installedPackage));
      assert(
        rejection instanceof PackageFileError &&
          rejection.kind === "missing" &&
          rejection.path === relativePath,
        `missing-file control returned the wrong structured error for ${relativePath}`,
      );
      console.log(`missing-file control rejected ${relativePath}`);
    } finally {
      await rename(hiddenFile, requiredFile);
    }
  }
}

async function resolveExecutable(command, required = true) {
  const candidates = path.isAbsolute(command) || path.win32.isAbsolute(command)
    ? [path.resolve(command)]
    : (process.env.PATH ?? "")
        .split(path.delimiter)
        .filter(Boolean)
        .map((entry) => path.join(entry, command));
  const canonicalRepoRoot = await realpath(repoRoot);

  for (const candidate of candidates) {
    try {
      await access(candidate, constants.X_OK);
      const canonicalCandidate = await realpath(candidate);
      if (
        isInside(repoRoot, path.resolve(candidate)) ||
        isInside(canonicalRepoRoot, canonicalCandidate)
      ) {
        continue;
      }
      return candidate;
    } catch (error) {
      if (error.code !== "ENOENT" && error.code !== "EACCES") {
        throw error;
      }
    }
  }
  if (required) throw new Error(`could not find executable: ${command}`);
}

async function buildConsumerPath(consumerRoot, executables) {
  const entries = [
    path.join(consumerRoot, "node_modules", ".bin"),
    ...executables.map((executable) => path.dirname(executable)),
    ...systemPathEntries,
  ].filter((entry, index, all) => all.indexOf(entry) === index);
  const canonicalRepoRoot = await realpath(repoRoot);

  for (const entry of [...entries, ...executables]) {
    const canonicalEntry = await realpath(entry);
    assert(
      !isInside(repoRoot, path.resolve(entry)) && !isInside(canonicalRepoRoot, canonicalEntry),
      `consumer PATH entry resolves into the ServoKit checkout: ${entry} -> ${canonicalEntry}`,
    );
  }

  return entries.join(path.delimiter);
}

async function assertOptionalUvPath(consumerRoot, tempRoot) {
  const shimDir = path.join(tempRoot, "uv-shim");
  await mkdir(shimDir);
  await symlink(path.join(exampleRoot, "android", "gradlew"), path.join(shimDir, "uv"));
  const inheritedPath = process.env.PATH;
  try {
    process.env.PATH = [shimDir, inheritedPath].filter(Boolean).join(path.delimiter);
    const uv = await resolveExecutable("uv", false).then((file) => file && realpath(file));
    if (!uv) {
      console.log("uv unavailable; retaining the upstream Python fallback");
      return;
    }
    const consumerPath = await buildConsumerPath(consumerRoot, [uv]);
    assert(consumerPath.split(path.delimiter).includes(path.dirname(uv)));
    assert(!consumerPath.split(path.delimiter).includes(shimDir), "checkout uv shim entered PATH");
    console.log(`canonical uv ${uv} is present in the consumer PATH`);
  } finally {
    process.env.PATH = inheritedPath;
  }
}

async function assertAutolinked(configPath, installedPackage) {
  const config = JSON.parse(await readFile(configPath, "utf8"));
  const dependency = config.dependencies?.["react-native-servokit"];
  assert(dependency, `${configPath} did not include react-native-servokit`);
  assert(
    (await realpath(dependency.root)) === (await realpath(installedPackage)),
    `${configPath} resolved react-native-servokit outside the installed package`,
  );
}

async function assertCanonicalFixtures(root) {
  const files = (await readdir(canonicalFixtureRoot, { recursive: true, withFileTypes: true }))
    .filter((entry) => entry.isFile())
    .map((entry) => path.relative(canonicalFixtureRoot, path.join(entry.parentPath, entry.name)))
    .filter((file) => file !== "README.md")
    .sort();
  await Promise.all(files.map((file) => assertRegularFile(root, file)));
  return files.length;
}

async function assertSyncedFixtureAssets(root) {
  const generatedFixtureRoot = path.join(
    root,
    "android/app/build/generated/servo-fixtures/assets/servo-fixtures",
  );
  const count = await assertCanonicalFixtures(generatedFixtureRoot);
  console.log(`syncFixtureAssets produced all ${count} canonical fixture files`);
}

async function listArchiveEntries(unzip, archive) {
  return (await runChecked(unzip, ["-Z1", archive], { capture: true }))
    .trim()
    .split(/\r?\n/)
    .filter(Boolean);
}

async function extractArchiveEntries(unzip, archive, outputRoot, entries) {
  await rm(outputRoot, { recursive: true, force: true });
  await mkdir(outputRoot, { recursive: true });
  await runChecked(unzip, ["-qq", archive, ...entries, "-d", outputRoot]);
  return Promise.all(entries.map((entry) => assertRegularFile(outputRoot, entry)));
}

async function assertElfArchitecture(fileTool, file, expected) {
  const output = await runChecked(fileTool, [file], { capture: true });
  assert.match(output, expected, `${file} has the wrong ELF architecture: ${output.trim()}`);
}

async function inspectAndroidAar(installedPackage, tempRoot, unzip, fileTool) {
  const aar = await assertRegularFile(
    installedPackage,
    androidAarPath,
  );
  const hostEntries = [
    "jni/arm64-v8a/libservokit_host_android.so",
    "jni/x86_64/libservokit_host_android.so",
  ];
  const entries = await listArchiveEntries(unzip, aar);
  assertExactAndroidJniEntries(entries);
  for (const entry of hostEntries) {
    assert(entries.includes(entry), `installed AAR is missing ${entry}`);
  }

  const extracted = await extractArchiveEntries(
    unzip,
    aar,
    path.join(tempRoot, "android-aar-inspection"),
    hostEntries,
  );
  await assertElfArchitecture(fileTool, extracted[0], /ARM aarch64/i);
  await assertElfArchitecture(fileTool, extracted[1], /x86[-_ ]64/i);
  const hostHashes = await Promise.all(extracted.map(sha256));
  console.log(
    `Android AAR sha256=${await sha256(aar)} arm64-v8a=${hostHashes[0]} x86_64=${hostHashes[1]}`,
  );
  return { arm64: hostHashes[0] };
}

async function assertNoAndroidFallback(consumerRoot, installedPackage) {
  const files = [
    path.join(consumerRoot, "android", "settings.gradle"),
    path.join(consumerRoot, "android", "app", "build.gradle"),
    path.join(installedPackage, "android", "build.gradle"),
    path.join(installedPackage, "package.json"),
    path.join(installedPackage, "react-native.config.js"),
  ];
  const forbidden = /\b(?:cargo|rustc|rustup|curl|wget)\b|Cargo\.toml|android\/servokit-android-host\/|crates\/|workspace:|\b(?:preinstall|install|postinstall)\b/i;
  for (const file of files) {
    const text = await readFile(file, "utf8");
    assert(!forbidden.test(text), `${path.relative(consumerRoot, file)} contains a native fallback`);
    assert(!text.includes(repoRoot), `${path.relative(consumerRoot, file)} references the workspace`);
  }
}

async function validateWorkspaceAndroidConfiguration() {
  const androidRoot = path.join(exampleRoot, "android");
  const workspaceDependency = path.join(exampleRoot, "node_modules", "react-native-servokit");
  assert((await lstat(workspaceDependency)).isSymbolicLink(), "workspace dependency is not a symlink");
  const output = await runChecked(
    path.join(androidRoot, "gradlew"),
    [
      "--no-daemon", "--no-build-cache", "--console=plain", "--rerun-tasks",
      ":app:syncFixtureAssets",
    ],
    { capture: true, echo: true, cwd: androidRoot },
  );
  assert(
    !output.includes(":servokit-android-host"),
    "workspace Android settings retained the manual source-host project",
  );
  await assertSyncedFixtureAssets(exampleRoot);
  console.log("workspace Android standard autolinking configuration and canonical fixtures passed");
}

async function validateAndroid(consumerRoot, installedPackage, tempRoot) {
  const androidRoot = path.join(consumerRoot, "android");
  const java = await resolveExecutable(
    process.env.JAVA_HOME ? path.join(process.env.JAVA_HOME, "bin", "java") : "java",
  );
  const node = await resolveExecutable(process.env.NODE_BINARY ?? process.execPath);
  const bun = await resolveExecutable("bun");
  const unzip = await resolveExecutable("unzip");
  const fileTool = await resolveExecutable("file");
  const toolPath = await buildConsumerPath(
    consumerRoot,
    [java, node, bun, unzip, fileTool],
  );
  const aarHosts = await inspectAndroidAar(installedPackage, tempRoot, unzip, fileTool);
  await assertNoAndroidFallback(consumerRoot, installedPackage);

  await runChecked(
    path.join(androidRoot, "gradlew"),
    [
      "--no-daemon",
      "--no-build-cache",
      "--console=plain",
      ":app:assembleDebug",
      "-PreactNativeArchitectures=arm64-v8a",
    ],
    {
      capture: true,
      echo: true,
      cwd: androidRoot,
      env: {
        ...process.env,
        GRADLE_USER_HOME: path.join(tempRoot, "gradle-home"),
        NODE_BINARY: node,
        PATH: toolPath,
      },
    },
  );

  await assertAutolinked(
    path.join(androidRoot, "build", "generated", "autolinking", "autolinking.json"),
    installedPackage,
  );
  await assertComponentOnlyCodegen(installedPackage);
  await assertSyncedFixtureAssets(consumerRoot);

  const apk = await assertRegularFile(consumerRoot, "android/app/build/outputs/apk/debug/app-debug.apk");
  const apkEntries = await listArchiveEntries(unzip, apk);
  const selectedHost = "lib/arm64-v8a/libservokit_host_android.so";
  assert.deepEqual(
    apkEntries.filter((entry) => entry.endsWith("/libservokit_host_android.so")),
    [selectedHost],
    "Android APK did not select only the requested arm64-v8a host",
  );
  const [apkHost] = await extractArchiveEntries(
    unzip,
    apk,
    path.join(tempRoot, "android-apk-inspection"),
    [selectedHost],
  );
  await assertElfArchitecture(fileTool, apkHost, /ARM aarch64/i);
  const apkHostHash = await sha256(apkHost);
  assert.equal(apkHostHash, aarHosts.arm64, "Android APK host does not match the packaged AAR");
  for (const removedLibrary of [
    "libreact-native-servokit.so",
    "libservokit_bindings.so",
  ]) {
    assert(
      !apkEntries.some((entry) => entry.endsWith(`/${removedLibrary}`)),
      `Android APK retained ${removedLibrary}`,
    );
  }
  console.log(
    `Android standard-autolink APK selected arm64-v8a host sha256=${apkHostHash} without UBRN libraries`,
  );
}

function asArray(value) {
  if (value === undefined) return [];
  return Array.isArray(value) ? value : [value];
}

async function inspectInstalledIosPod(iosRoot, installedPackage) {
  const installedSpecPath = path.join(iosRoot, "Pods", "Local Podspecs", "Servokit.podspec.json");
  const installedSpec = JSON.parse(await readFile(installedSpecPath, "utf8"));
  assert.deepEqual(asArray(installedSpec.source_files), ["ios/ServoView.mm"]);
  assert.deepEqual(asArray(installedSpec.frameworks), ["WebKit"]);
  assert.deepEqual(
    asArray(installedSpec.vendored_frameworks),
    ["ios/ServoKitController.xcframework"],
  );
  assert.deepEqual(Object.keys(installedSpec.platforms ?? {}), ["ios"]);
  assert.equal(installedSpec.subspecs, undefined, "installed iOS pod retained subspecs");

  const podfileLock = await readFile(path.join(iosRoot, "Podfile.lock"), "utf8");
  assert(
    podfileLock.includes("Servokit (from `../node_modules/react-native-servokit`)"),
    "CocoaPods did not resolve Servokit from installed node_modules",
  );
  assert(!podfileLock.includes("Servokit (from `../..`)"), "CocoaPods resolved the workspace package");

  const activeFiles = [
    path.join(iosRoot, "Podfile"),
    installedSpecPath,
    path.join(installedPackage, "Servokit.podspec"),
    path.join(installedPackage, "package.json"),
    path.join(installedPackage, "react-native.config.js"),
  ];
  const forbidden = /\b(?:cargo|rustc|rustup|curl|wget)\b|Cargo\.toml|servokit-host-android|workspace:|https?:\/\/.*\.(?:zip|xcframework)\b|(?:prepare|preinstall|install|postinstall)_command|script_phase/i;
  for (const file of activeFiles) {
    const text = await readFile(file, "utf8");
    assert(!forbidden.test(text), `${path.relative(iosRoot, file)} contains a source/download hook`);
    assert(!text.includes(repoRoot), `${path.relative(iosRoot, file)} references the workspace`);
  }
  await assertRegularFile(installedPackage, "ios/ServoView.mm");
}

async function assertMachOArchitecture(lipo, executable, architecture) {
  const output = await runChecked(lipo, ["-archs", executable], { capture: true });
  assert.equal(output.trim(), architecture, `${executable} contains ${output.trim()}, not ${architecture}`);
}

function assertControllerSymbolsFromArchive(linkMapText, label) {
  for (const symbol of controllerSymbols) {
    const match = linkMapText.match(
      new RegExp(`^.*\\[\\s*(\\d+)\\].*_${symbol}(?:\\s|$).*$`, "m"),
    );
    assert(match, `${label} link map is missing ${symbol}`);
    assert(
      new RegExp(
        `^\\[\\s*${match[1]}\\]\\s+.*libservokit_controller_ffi\\.a(?:\\(|/|$)`,
        "m",
      ).test(linkMapText),
      `${label} did not link ${symbol} from libservokit_controller_ffi.a`,
    );
  }
}

async function validateIosMatrix({
  installedPackage,
  iosRoot,
  matrix,
  tempRoot,
  env,
  xcodebuild,
  lipo,
  nm,
  otool,
}) {
  const buildRoot = path.join(tempRoot, `xcode-${matrix.label}`);
  const linkMap = path.join(tempRoot, `ServoKitExample-${matrix.label}.linkmap`);
  try {
    await runChecked(
      xcodebuild,
      [
        "-workspace",
        path.join(iosRoot, "ServoKitExample.xcworkspace"),
        "-scheme",
        "ServoKitExample",
        "-configuration",
        "Release",
        "-sdk",
        matrix.sdk,
        "-destination",
        matrix.destination,
        "-derivedDataPath",
        buildRoot,
        `ARCHS=${matrix.architecture}`,
        `ONLY_ACTIVE_ARCH=${matrix.onlyActiveArchitecture}`,
        "CODE_SIGNING_ALLOWED=NO",
        "LD_GENERATE_MAP_FILE=YES",
        "LD_MAP_FILE_PATH=$(TARGET_TEMP_DIR)/$(PRODUCT_NAME)-LinkMap-$(CURRENT_ARCH).txt",
        "build",
      ],
      { cwd: iosRoot, env },
    );

    const executable = await assertRegularFile(
      buildRoot,
      `Build/Products/${matrix.productDirectory}/ServoKitExample.app/ServoKitExample`,
    );
    await assertMachOArchitecture(lipo, executable, matrix.architecture);

    const selectedArchives = await findFiles(
      path.join(buildRoot, "Build/Products"),
      "libservokit_controller_ffi.a",
    );
    assert.equal(selectedArchives.length, 1, `${matrix.label} selected ${selectedArchives.length} archives`);
    const expectedArchive = await assertRegularFile(
      installedPackage,
      `ios/ServoKitController.xcframework/${matrix.slice}/libservokit_controller_ffi.a`,
    );
    const selectedHash = await sha256(selectedArchives[0]);
    const expectedHash = await sha256(expectedArchive);
    assert.equal(selectedHash, expectedHash, `${matrix.label} selected the wrong XCFramework slice`);

    const executableSymbols = await runChecked(nm, ["-g", executable], { capture: true });
    const builtLinkMaps = await findFiles(
      buildRoot,
      `ServoKitExample-LinkMap-${matrix.architecture}.txt`,
    );
    assert.equal(builtLinkMaps.length, 1, `${matrix.label} did not generate one app link map`);
    await cp(builtLinkMaps[0], linkMap);
    const linkMapText = await readFile(linkMap, "utf8");
    for (const symbol of controllerSymbols) {
      assert(executableSymbols.includes(`_${symbol}`), `${matrix.label} executable is missing ${symbol}`);
    }
    assertControllerSymbolsFromArchive(linkMapText, matrix.label);
    const dynamicLibraries = await runChecked(otool, ["-L", executable], { capture: true });
    assert(!/servo(?:kit)?[^/\s]*\.(?:dylib|framework)/i.test(dynamicLibraries), `${matrix.label} linked dynamic Servo`);
    console.log(
      `${matrix.label} arch=${matrix.architecture} slice=${matrix.slice} controller=${selectedHash} executable=${await sha256(executable)} linkmap=${await sha256(linkMap)}`,
    );
  } finally {
    await rm(buildRoot, { recursive: true, force: true });
  }
}

async function validateIos(consumerRoot, installedPackage, tempRoot) {
  const iosRoot = path.join(consumerRoot, "ios");
  const pod = await resolveExecutable("pod");
  const xcodebuild = await resolveExecutable("xcodebuild");
  const lipo = await resolveExecutable("lipo");
  const nm = await resolveExecutable("nm");
  const otool = await resolveExecutable("otool");
  const node = await resolveExecutable(process.env.NODE_BINARY ?? process.execPath);
  const bun = await resolveExecutable("bun");
  const consumerPath = await buildConsumerPath(
    consumerRoot,
    [pod, xcodebuild, lipo, nm, otool, node, bun],
  );
  const env = {
    ...process.env,
    CP_HOME_DIR: path.join(tempRoot, "cocoapods-home"),
    NODE_BINARY: node,
    PATH: consumerPath,
    RCT_NEW_ARCH_ENABLED: "1",
  };
  await runChecked(pod, ["install"], {
    capture: true,
    echo: true,
    cwd: iosRoot,
    env,
  });

  await assertAutolinked(
    path.join(iosRoot, "build", "generated", "autolinking", "autolinking.json"),
    installedPackage,
  );
  await assertRegularFile(iosRoot, "build/generated/ios/ReactCodegen/ServoViewSpec/ServoViewSpec-generated.mm");
  await assertRegularFile(iosRoot, "build/generated/ios/ReactCodegen/ServoViewSpec/ServoViewSpec.h");
  await inspectInstalledIosPod(iosRoot, installedPackage);

  for (const matrix of iosMatrices) {
    await validateIosMatrix({
      installedPackage,
      iosRoot,
      matrix,
      tempRoot,
      env,
      xcodebuild,
      lipo,
      nm,
      otool,
    });
  }
  console.log("iOS exact-package device-arm64, simulator-arm64, and simulator-x86_64 proofs passed");
}

function formatError(error, indent = "") {
  const lines = [`${indent}${error.name ?? "Error"}: ${error.message ?? String(error)}`];
  if (error instanceof AggregateError) {
    for (const nested of error.errors) {
      lines.push(formatError(nested, `${indent}  `));
    }
  } else if (error.cause) {
    lines.push(formatError(error.cause, `${indent}  `));
  }
  return lines.join("\n");
}

async function validate() {
  if (signalLifecycleChild) {
    await runSignalLifecycleChildMode();
    return;
  }

  const selectedModes = [negativeControlsOnly, iosOnly, lifecycleSelfCheckOnly].filter(Boolean);
  assert(
    selectedModes.length <= 1,
    "--negative-controls-only, --ios-only, and --lifecycle-self-check-only are mutually exclusive",
  );
  const retainedRoot = process.env.SERVOKIT_PACKED_CONSUMER_ROOT;
  assert(
    retainedRoot === undefined || !lifecycleSelfCheckOnly,
    "SERVOKIT_PACKED_CONSUMER_ROOT cannot be used with --lifecycle-self-check-only",
  );

  await selfCheckLifecycle();
  await selfCheckSignals();
  await selfCheckHermeticPaths();
  await assertExampleConfiguration(exampleRoot, packageRoot, true);
  if (lifecycleSelfCheckOnly) {
    return;
  }
  if (negativeControlsOnly) {
    await validateWorkspaceAndroidConfiguration();
  }

  let tempRoot;
  if (retainedRoot !== undefined) {
    assert(path.isAbsolute(retainedRoot), "SERVOKIT_PACKED_CONSUMER_ROOT must be an absolute path");
    assert(!(await exists(retainedRoot)), "SERVOKIT_PACKED_CONSUMER_ROOT must not already exist");
    await mkdir(path.dirname(retainedRoot), { recursive: true });
    await mkdir(retainedRoot);
    tempRoot = retainedRoot;
  } else {
    tempRoot = await mkdtemp(path.join(tmpdir(), "servokit-packed-consumer-"));
  }

  await runProtectedValidation(
    tempRoot,
    packageRoot,
    generatedPackagePaths,
    async () => {
      const tarball = await packPackage(tempRoot);
      const consumerRoot = path.join(tempRoot, "consumer");
      const stagedTarball = await stageConsumer(tarball, consumerRoot);
      const installedPackage = await installConsumer(consumerRoot, tempRoot, stagedTarball);
      await runMissingFileControls(installedPackage);

      if (negativeControlsOnly) {
        await assertOptionalUvPath(consumerRoot, tempRoot);
      } else if (iosOnly) {
        await validateIos(consumerRoot, installedPackage, tempRoot);
      } else {
        await validateAndroid(consumerRoot, installedPackage, tempRoot);
        await validateIos(consumerRoot, installedPackage, tempRoot);
      }
    },
    { retainTemp: retainedRoot !== undefined },
  );

  console.log(
    negativeControlsOnly
      ? "packed consumer focused controls passed"
      : iosOnly
        ? "packed external iOS consumer validation passed"
        : "packed external-consumer validation passed",
  );
}

validate().catch((error) => {
  console.error(formatError(error));
  process.exitCode = signalExitCodes[receivedSignal] ?? 1;
});
