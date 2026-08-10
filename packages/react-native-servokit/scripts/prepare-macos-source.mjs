#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import {
  accessSync,
  constants,
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  realpathSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const owner = 'react-native-servokit:macos-source:v1\n';
const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const outputRoot = join(packageRoot, '.servokit-source');
const currentOutput = join(outputRoot, 'current');
const buildInputPathspecs = [
  ':(top).cargo/',
  ':(top)rust-toolchain',
  ':(top)rust-toolchain.toml',
  ':(top)crates/Cargo.lock',
  ':(top)crates/Cargo.toml',
  ':(top)crates/servokit/',
  ':(top)crates/servokit-embedder/',
  ':(top)crates/servokit-host/',
  ':(top)crates/servokit-host-desktop/',
  ':(top)crates/vendor/tikv-jemalloc-sys/',
  ':(top)distribution/macos/build-xcframework.sh',
  ':(top)packages/react-native-servokit/package.json',
];

function capture(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: 'utf8',
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    const detail = (result.stderr || result.stdout).trim();
    throw new Error(detail || `${command} exited with status ${result.status}`);
  }
  return result.stdout.trim();
}

function run(command, args, options) {
  const result = spawnSync(command, args, options);
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`${command} exited with status ${result.status}`);
  }
}

function ownerMarker(directory) {
  return join(directory, '.servokit-source-owner');
}

function assertOwned(directory) {
  let marker;
  try {
    const stats = lstatSync(directory);
    if (!stats.isDirectory() || stats.isSymbolicLink()) {
      throw new Error();
    }
    marker = readFileSync(ownerMarker(directory), 'utf8');
  } catch {
    marker = undefined;
  }
  if (marker !== owner) {
    throw new Error(`refusing to replace unowned output: ${directory}`);
  }
}

function removeOwned(directory) {
  if (!existsSync(directory)) {
    return;
  }
  assertOwned(directory);
  rmSync(directory, { recursive: true });
}

function processIsRunning(pid) {
  if (!Number.isSafeInteger(pid) || pid <= 0) {
    return false;
  }
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error.code !== 'ESRCH';
  }
}

function claimBuildWorkspace(sourceRoot) {
  const key = createHash('sha256')
    .update(
      [realpathSync(packageRoot), realpathSync(sourceRoot)].join('\0')
    )
    .digest('hex');
  const directory = join(
    tmpdir(),
    `react-native-servokit-macos-source-${key}`
  );
  const pidMarker = join(directory, '.servokit-source-pid');

  if (existsSync(directory)) {
    assertOwned(directory);
    let pid;
    try {
      pid = Number(readFileSync(pidMarker, 'utf8').trim());
    } catch {}
    if (processIsRunning(pid)) {
      throw new Error(
        `ServoKit macOS source preparation is already running (${pid})`
      );
    }
    removeOwned(directory);
  }

  try {
    mkdirSync(directory);
  } catch (error) {
    if (error.code === 'EEXIST') {
      throw new Error('ServoKit macOS source preparation started concurrently');
    }
    throw error;
  }
  writeFileSync(ownerMarker(directory), owner, { flag: 'wx' });
  writeFileSync(pidMarker, `${process.pid}\n`, { flag: 'wx' });
  return directory;
}

function ensureOutputRoot() {
  if (existsSync(outputRoot)) {
    assertOwned(outputRoot);
    return;
  }
  mkdirSync(outputRoot);
  writeFileSync(ownerMarker(outputRoot), owner, { flag: 'wx' });
}

function readCurrentIdentity() {
  if (!existsSync(currentOutput)) {
    return undefined;
  }
  assertOwned(currentOutput);
  try {
    return JSON.parse(
      readFileSync(join(currentOutput, 'identity.json'), 'utf8')
    );
  } catch {
    return undefined;
  }
}

function sourceIdentity(sourceRoot, packageVersion) {
  const sourcePackagePath = join(
    sourceRoot,
    'packages/react-native-servokit/package.json'
  );
  const sourceManifestPath = join(sourceRoot, 'crates/Cargo.toml');
  const sourceHeaderPath = join(
    sourceRoot,
    'crates/servokit-host-desktop/include/servokit_desktop_private.h'
  );
  const sourceBuilder = join(
    sourceRoot,
    'distribution/macos/build-xcframework.sh'
  );

  try {
    for (const path of [
      sourcePackagePath,
      sourceManifestPath,
      sourceHeaderPath,
      sourceBuilder,
    ]) {
      accessSync(path, constants.R_OK);
    }
    accessSync(sourceBuilder, constants.X_OK);
  } catch {
    throw new Error('SERVOKIT_SOURCE_DIR must point to a ServoKit checkout');
  }

  const sourcePackage = JSON.parse(readFileSync(sourcePackagePath, 'utf8'));
  if (sourcePackage.version !== packageVersion) {
    throw new Error(
      `SERVOKIT_SOURCE_DIR version ${sourcePackage.version} does not match ${packageVersion}`
    );
  }

  const gitRoot = realpathSync(
    capture('git', ['rev-parse', '--show-toplevel'], sourceRoot)
  );
  if (gitRoot !== sourceRoot) {
    throw new Error('SERVOKIT_SOURCE_DIR must point to the ServoKit checkout root');
  }
  if (
    capture(
      'git',
      [
        'status',
        '--porcelain=v1',
        '--untracked-files=all',
        '--ignored=matching',
        '--',
        ...buildInputPathspecs,
      ],
      sourceRoot
    )
  ) {
    throw new Error(
      'SERVOKIT_SOURCE_DIR must contain clean macOS build inputs'
    );
  }

  return {
    identity: {
      schema: 1,
      packageVersion,
      sourceCommit: capture('git', ['rev-parse', 'HEAD^{commit}'], sourceRoot),
    },
    sourceBuilder,
  };
}

function sameIdentity(left, right) {
  return (
    left?.schema === right.schema &&
    left?.packageVersion === right.packageVersion &&
    left?.sourceCommit === right.sourceCommit
  );
}

function verifyXCFramework(framework, stdio) {
  accessSync(join(framework, 'Info.plist'), constants.R_OK);
  run(
    '/usr/bin/codesign',
    ['--verify', '--deep', '--strict', framework],
    { stdio }
  );
}

function isValidXCFramework(framework) {
  try {
    verifyXCFramework(framework, 'ignore');
    return true;
  } catch {
    return false;
  }
}

function stageFramework(sourceFramework, identity) {
  ensureOutputRoot();
  const suffix = randomUUID();
  const nextOutput = join(outputRoot, `.next-${suffix}`);
  const previousOutput = join(outputRoot, `.previous-${suffix}`);
  let movedPrevious = false;

  mkdirSync(nextOutput);
  writeFileSync(ownerMarker(nextOutput), owner, { flag: 'wx' });

  try {
    run(
      '/usr/bin/ditto',
      [
        '--rsrc',
        '--extattr',
        '--acl',
        sourceFramework,
        join(nextOutput, 'ServoKit.xcframework'),
      ],
      { stdio: 'inherit' }
    );
    verifyXCFramework(
      join(nextOutput, 'ServoKit.xcframework'),
      'inherit'
    );
    writeFileSync(
      join(nextOutput, 'identity.json'),
      `${JSON.stringify(identity, null, 2)}\n`
    );

    if (existsSync(currentOutput)) {
      assertOwned(currentOutput);
      renameSync(currentOutput, previousOutput);
      movedPrevious = true;
    }
    try {
      renameSync(nextOutput, currentOutput);
    } catch (error) {
      if (movedPrevious) {
        renameSync(previousOutput, currentOutput);
        movedPrevious = false;
      }
      throw error;
    }
    if (movedPrevious) {
      removeOwned(previousOutput);
    }
  } finally {
    removeOwned(nextOutput);
    if (existsSync(previousOutput) && !existsSync(currentOutput)) {
      renameSync(previousOutput, currentOutput);
    } else if (existsSync(previousOutput)) {
      removeOwned(previousOutput);
    }
  }
}

function main() {
  if (process.env.SERVOKIT_BUILD_FROM_SOURCE !== '1') {
    return;
  }
  if (process.platform !== 'darwin') {
    throw new Error('ServoKit macOS source preparation requires macOS');
  }

  const sourceDirectory = process.env.SERVOKIT_SOURCE_DIR;
  if (!sourceDirectory) {
    throw new Error(
      'SERVOKIT_SOURCE_DIR is required when SERVOKIT_BUILD_FROM_SOURCE=1'
    );
  }

  let sourceRoot;
  try {
    sourceRoot = realpathSync(sourceDirectory);
  } catch {
    throw new Error('SERVOKIT_SOURCE_DIR must point to a ServoKit checkout');
  }
  const packageVersion = JSON.parse(
    readFileSync(join(packageRoot, 'package.json'), 'utf8')
  ).version;
  const { identity, sourceBuilder } = sourceIdentity(
    sourceRoot,
    packageVersion
  );
  const stagedFramework = join(currentOutput, 'ServoKit.xcframework');
  if (
    sameIdentity(readCurrentIdentity(), identity) &&
    isValidXCFramework(stagedFramework)
  ) {
    console.log(`ServoKit macOS source framework is current (${identity.sourceCommit})`);
    return;
  }

  const temporaryRoot = claimBuildWorkspace(sourceRoot);
  try {
    const artifactDirectory = join(temporaryRoot, '.servokit-source');
    run(sourceBuilder, [], {
      cwd: sourceRoot,
      env: {
        ...process.env,
        CARGO_TARGET_DIR: join(temporaryRoot, 'cargo-target'),
        SERVOKIT_ARTIFACT_DIR: artifactDirectory,
        SERVOKIT_FRAMEWORK_ONLY: '1',
      },
      stdio: 'inherit',
    });
    const builtFramework = join(
      artifactDirectory,
      'stage',
      'ServoKit.xcframework'
    );
    accessSync(join(builtFramework, 'Info.plist'), constants.R_OK);
    stageFramework(builtFramework, identity);
  } finally {
    removeOwned(temporaryRoot);
  }

  console.log(`Prepared ServoKit macOS source framework (${identity.sourceCommit})`);
}

try {
  main();
} catch (error) {
  console.error(`ServoKit source preparation failed: ${error.message}`);
  process.exitCode = 1;
}
