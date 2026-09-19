#!/usr/bin/env node

import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const scriptDir = dirname(fileURLToPath(import.meta.url));

const options = {
  packageRoot: resolve(scriptDir, ".."),
  exampleRoot: resolve(scriptDir, "..", "example"),
  requireWindowsProject: false,
};

for (let index = 2; index < process.argv.length; index += 1) {
  const arg = process.argv[index];
  if (arg === "--package-root") {
    options.packageRoot = resolve(process.argv[++index] ?? "");
  } else if (arg === "--example-root") {
    options.exampleRoot = resolve(process.argv[++index] ?? "");
  } else if (arg === "--require-windows-project") {
    options.requireWindowsProject = true;
  } else {
    fail(`Unknown argument: ${arg}`);
  }
}

function fail(message, details) {
  console.error(`[servokit-windows-autolink] ${message}`);
  if (details) {
    console.error(details);
  }
  process.exit(1);
}

function assert(condition, message, details) {
  if (!condition) {
    fail(message, details);
  }
}

function loadJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function normalizePath(value) {
  return resolve(value).toLowerCase();
}

function normalizeProjectPath(value) {
  return String(value).replaceAll("\\", "/");
}

function extractFirstJsonObject(output) {
  const start = output.indexOf("{");
  if (start === -1) {
    return null;
  }

  let depth = 0;
  let inString = false;
  let escaping = false;

  for (let index = start; index < output.length; index += 1) {
    const char = output[index];
    if (inString) {
      if (escaping) {
        escaping = false;
      } else if (char === "\\") {
        escaping = true;
      } else if (char === "\"") {
        inString = false;
      }
      continue;
    }

    if (char === "\"") {
      inString = true;
    } else if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return output.slice(start, index + 1);
      }
    }
  }

  return null;
}

function runReactNativeConfig(exampleRoot) {
  const binDir = join(exampleRoot, "node_modules", ".bin");
  const binNames =
    process.platform === "win32"
      ? ["react-native.exe", "react-native.cmd"]
      : ["react-native"];
  const reactNativeBin = binNames
    .map((binName) => join(binDir, binName))
    .find((candidate) => existsSync(candidate));
  assert(
    reactNativeBin,
    `React Native CLI binary was not found in ${binDir}`
  );

  const isCommandScript = reactNativeBin.endsWith(".cmd");
  const command = isCommandScript
    ? process.env.ComSpec || "cmd.exe"
    : reactNativeBin;
  const args = isCommandScript
    ? ["/d", "/s", "/c", `"${reactNativeBin}" config`]
    : ["config"];

  const result = spawnSync(command, args, {
    cwd: exampleRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      CI: process.env.CI ?? "1",
    },
    maxBuffer: 64 * 1024 * 1024,
  });

  assert(
    result.status === 0,
    `react-native config exited with ${result.status}`,
    `${result.stdout}\n${result.stderr}`.trim()
  );

  const jsonText = extractFirstJsonObject(result.stdout);
  assert(
    jsonText,
    "react-native config did not emit a JSON object on stdout",
    `${result.stdout}\n${result.stderr}`.trim()
  );

  try {
    return JSON.parse(jsonText);
  } catch (error) {
    fail(
      `react-native config emitted malformed JSON: ${error.message}`,
      `${result.stdout}\n${result.stderr}`.trim()
    );
  }
}

if (process.platform !== "win32") {
  fail(
    "React Native Windows dependency config must be proven on Windows because RNW returns null for Windows dependencies on non-win32 hosts."
  );
}

const exampleManifest = loadJson(join(options.exampleRoot, "package.json"));
assert(
  exampleManifest.dependencies?.["react-native-windows"],
  "The example app must list react-native-windows as a dependency so the RN CLI loads the Windows platform."
);
assert(
  !exampleManifest.devDependencies?.["react-native-windows"],
  "react-native-windows must not be hidden in example devDependencies for this source-consumer proof."
);

const config = runReactNativeConfig(options.exampleRoot);
const dependency = config.dependencies?.["react-native-servokit"];
assert(dependency, "react-native config did not discover react-native-servokit.");
assert(
  typeof dependency.root === "string",
  "react-native config did not include a root for react-native-servokit."
);
assert(
  normalizePath(dependency.root) === normalizePath(options.packageRoot),
  "react-native config resolved react-native-servokit from the wrong package root.",
  `expected=${options.packageRoot}\nactual=${dependency.root}`
);

const windows = dependency.platforms?.windows;
assert(windows, "react-native config did not expose the ServoKit Windows dependency.");
assert(
  typeof windows.folder === "string",
  "ServoKit Windows dependency did not include a folder."
);
assert(
  normalizePath(windows.folder) === normalizePath(options.packageRoot),
  "ServoKit Windows dependency folder is not the package root.",
  `expected=${options.packageRoot}\nactual=${windows.folder}`
);
assert(windows.sourceDir === "windows", "ServoKit Windows sourceDir is not windows.");
assert(
  windows.solutionFile === "ServoKit.sln",
  "ServoKit Windows solutionFile is not ServoKit.sln."
);
assert(
  Array.isArray(windows.projects) && windows.projects.length === 1,
  "ServoKit Windows dependency must expose exactly one direct project."
);

const [project] = windows.projects;
assert(
  normalizeProjectPath(project.projectFile) === "ServoKit/ServoKit.vcxproj",
  "ServoKit Windows projectFile is not ServoKit/ServoKit.vcxproj.",
  String(project.projectFile)
);
assert(project.directDependency === true, "ServoKit Windows project is not direct.");
assert(project.projectName === "ServoKit", "ServoKit Windows projectName is wrong.");
assert(project.projectLang === "cpp", "ServoKit Windows project is not C++.");
assert(
  project.projectGuid && !String(project.projectGuid).startsWith("Error:"),
  "ServoKit Windows projectGuid was not resolved."
);
assert(
  project.cppHeaders?.includes("winrt/ServoKit.h"),
  "ServoKit Windows dependency did not expose its C++/WinRT header."
);
assert(
  project.cppPackageProviders?.includes("ServoKit::ReactPackageProvider"),
  "ServoKit Windows dependency did not expose its package provider."
);
assert(config.platforms?.windows, "React Native Windows did not register a Windows platform.");

const windowsCommands = (config.commands ?? [])
  .map((command) => command.name)
  .filter((name) => name.includes("windows"))
  .sort();
assert(
  windowsCommands.includes("run-windows"),
  "React Native Windows did not register run-windows."
);
assert(
  windowsCommands.includes("autolink-windows"),
  "React Native Windows did not register autolink-windows."
);

let appWindows = null;
if (options.requireWindowsProject) {
  appWindows = config.project?.windows;
  assert(
    appWindows,
    "react-native config did not expose a React Native Windows app project."
  );
  assert(
    typeof appWindows.folder === "string",
    "React Native Windows app project did not include a folder."
  );
  assert(
    normalizePath(appWindows.folder) === normalizePath(options.exampleRoot),
    "React Native Windows app project folder is not the app root.",
    `expected=${options.exampleRoot}\nactual=${appWindows.folder}`
  );
  assert(
    appWindows.sourceDir === "windows",
    "React Native Windows app project sourceDir is not windows."
  );
  assert(
    typeof appWindows.solutionFile === "string" &&
      appWindows.solutionFile.endsWith(".sln") &&
      !appWindows.solutionFile.startsWith("Error:"),
    "React Native Windows app project solutionFile was not resolved.",
    String(appWindows.solutionFile)
  );
  assert(
    appWindows.project,
    "React Native Windows app project did not include a project."
  );
  assert(
    normalizeProjectPath(appWindows.project.projectFile).endsWith(".vcxproj") &&
      !normalizeProjectPath(appWindows.project.projectFile).startsWith("Error:"),
    "React Native Windows app project projectFile was not resolved.",
    String(appWindows.project.projectFile)
  );
  assert(
    typeof appWindows.project.projectName === "string" &&
      appWindows.project.projectName.length > 0,
    "React Native Windows app project projectName was not resolved."
  );
  assert(
    appWindows.project.projectLang === "cpp",
    "React Native Windows app project is not C++."
  );
  assert(
    appWindows.project.projectGuid &&
      !String(appWindows.project.projectGuid).startsWith("Error:"),
    "React Native Windows app project projectGuid was not resolved."
  );
}

console.log(
  JSON.stringify(
    {
      status: "passed",
      exampleRoot: options.exampleRoot,
      packageRoot: options.packageRoot,
      platformKeys: Object.keys(config.platforms ?? {}).sort(),
      windowsCommands,
      servokitWindows: {
        folder: windows.folder,
        sourceDir: windows.sourceDir,
        solutionFile: windows.solutionFile,
        projectFile: project.projectFile,
        projectName: project.projectName,
        projectLang: project.projectLang,
        projectGuid: project.projectGuid,
        directDependency: project.directDependency,
        cppHeaders: project.cppHeaders,
        cppPackageProviders: project.cppPackageProviders,
      },
      ...(appWindows
        ? {
            appWindows: {
              folder: appWindows.folder,
              sourceDir: appWindows.sourceDir,
              solutionFile: appWindows.solutionFile,
              projectFile: appWindows.project.projectFile,
              projectName: appWindows.project.projectName,
              projectLang: appWindows.project.projectLang,
              projectGuid: appWindows.project.projectGuid,
            },
          }
        : {}),
    },
    null,
    2
  )
);
