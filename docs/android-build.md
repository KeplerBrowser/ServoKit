# Android build

## Producer prerequisites

- `cargo ndk`
- Rust Android targets: `aarch64-linux-android` and `x86_64-linux-android`
- Android NDK `28.2.13676358` (the version configured in `crates/servokit-host-android/android/build.gradle`)
- JDK 17
- Canonical `uv` when available, or Python >=3.11 as a fallback for Servo source generation

These prerequisites build the Android host AAR in the ServoKit checkout. An
installed `react-native-servokit` consumer uses the packaged AAR and does not
need Cargo, Rust targets, Servo source-generation tools, or the producer Gradle
project. It still needs the normal React Native Android SDK, JDK, and native
Codegen toolchain required by its app.

## Android alpha matrix

| Surface | Supported alpha value |
| --- | --- |
| ABI | `arm64-v8a`, `x86_64` |
| Rust target | `aarch64-linux-android`, `x86_64-linux-android` |
| Android API | Library `minSdkVersion` 24; AAR metadata `minCompileSdk` 36; `targetSdkVersion` is app-owned (repo examples use 36) |
| Producer Android NDK | `28.2.13676358` |
| Tested repo tuple | JDK 17, Gradle wrapper 9.3.1, Android Gradle Plugin 8.12.0, Kotlin Gradle Plugin 2.1.20; Java/Kotlin bytecode targets 17 |
| Official minimum floors | AGP 8.12 requires Gradle 8.13 and JDK 17; Android API 36 requires AGP 8.9.1 |
| Build host evidence | macOS build validated; Windows NDK `.exe`/`.cmd` path resolution configuration-tested, without an end-to-end Windows build claim |

The producer Gradle module intentionally excludes unsupported ABIs and fails early if the
Gradle/AGP floor or tested JDK is not met, Cargo, `cargo ndk`, or rustup is
missing, either Rust Android target is not installed, or the configured NDK
LLVM toolchain cannot be found. Android's manifest merger and AAR metadata
enforce the consumer minimum and compile SDK respectively. The
minimum compatibility floors are not a claim that untested future tool versions
are supported.

The producer ABI set does not replace app-level release configuration.
Consumer apps select `arm64-v8a`, `x86_64`, or both through standard React
Native Android architecture configuration. The package does not impose its own
ABI filter.

## What the build does

The crate-owned reusable `crates/servokit-host-android/android` Gradle library
module invokes `cargo ndk` for both supported ABIs and packages
`libservokit_host_android.so`, `libc++_shared.so`, and the proof-only Kotlin/JNI
host wrapper into one release AAR. The canonical
`verifyServokitAndroidReleaseAar` task depends on `bundleReleaseAar` and checks
the exact AAR layout and native closure. `stageReactNativeServokitReleaseAar`
depends on that verifier and copies its output into the React Native package.

React Native mounted commands reach the packaged host through the generated
Fabric component and the package's thin Kotlin adapter; no separate React
Native TurboModule library is linked. The repository-local Cargo patch strategy
for the Servo graph lives in one place:
[`rust-dependency-baseline.md`](./rust-dependency-baseline.md).

That baseline doc is the canonical place to track the current Servo 0.3.0 pin,
shared Rustls provider expectations, the `tikv-jemalloc-sys` Android toolchain
workaround, and the temporary GPUI-only `stylo_derive` 0.18.0 patch.

## Local React Native example app

The React Native package's Android module consumes the staged AAR with an
ordinary local file dependency. Repository maintainers can verify and stage a
new producer artifact before building the example:

```sh
bun install
examples/react-native-app/android/gradlew \
  -p examples/react-native-app/android \
  :servokit-android-host:stageReactNativeServokitReleaseAar
bun run --cwd examples/react-native-app build:android
```

The staging command is producer work. A package installed from an npm archive
already contains the AAR and does not run it.

## Kotlin Android browser example

`examples/android-kotlin-browser` is an experimental app-facing Kotlin Android
browser example. It uses the shared `:servokit-android-host` Gradle module and
shared host/control coordinator (`ServoViewBinding`, surface lifecycle helper,
typed events, and picker helpers) without loading React Native. It is not a
stable Kotlin SDK or public AAR.

Build it from the repository root with:

```sh
examples/react-native-app/android/gradlew \
  -p examples/android-kotlin-browser \
  :app:assembleDebug
```

Manual run instructions live in
[`examples/android-kotlin-browser`](../examples/android-kotlin-browser/README.md).

## Native Android proof app

`examples/android-native-example` is an experimental proof app that creates a
native Android `SurfaceView`, depends on the shared `:servokit-android-host`
Gradle library, renders the shared smoke fixture initial URL, and logs shared
URL/load events without loading React Native. It is proof-app glue only, not a
stable Kotlin SDK or reusable Android API.

Build it from the repository root with:

```sh
examples/react-native-app/android/gradlew \
  -p examples/android-native-example \
  :app:assembleDebug
```

Manual run instructions and expected logcat events live in
[`examples/android-native-example`](../examples/android-native-example/README.md).

The focused native verification commands are:

```sh
cd crates
cargo test --workspace
cargo test -p servokit-host-android

bun run --cwd examples/react-native-app test:native:android
```

The native Android proof apps above continue to consume the source-side host
project and therefore need the producer Rust and NDK prerequisites. Packed
React Native consumers are different: normal React Native autolinking resolves
`node_modules/react-native-servokit/android`, whose Gradle module depends only
on its package-local AAR. It contains no copied Rust workspace, host-project
include, native download, or consumer Cargo build. Maven publication is not
part of this package route.

## Scope note

This page only covers the Android Servo-backed build path. The packaged AAR
contains `arm64-v8a` and `x86_64`; runtime acceptance and broader platform
posture are separate checks. Package API and cross-platform posture live in
[`react-native-servokit.md`](./react-native-servokit.md).
