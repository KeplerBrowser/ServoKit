#![cfg(any(target_os = "macos", target_os = "windows"))]

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::process::Stdio;
use std::process::{Command, Output};
#[cfg(target_os = "windows")]
use std::thread;
#[cfg(target_os = "windows")]
use std::time::{Duration, Instant};

const PACKAGE: &str = "servokit-host-desktop";

#[cfg(target_os = "macos")]
const STATIC_LIBRARY: &str = "libservokit_host_desktop.a";
#[cfg(all(target_os = "windows", target_env = "msvc"))]
const STATIC_LIBRARY: &str = "servokit_host_desktop.lib";
#[cfg(all(target_os = "windows", target_env = "gnu"))]
const STATIC_LIBRARY: &str = "libservokit_host_desktop.a";

#[test]
fn c_boundary() {
    let crate_dir = env::var_os("SERVOKIT_REPO_ROOT")
        .map(PathBuf::from)
        .map(|repo_root| repo_root.join("crates").join(PACKAGE))
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let workspace_dir = crate_dir.parent().expect("crate must be in the workspace");
    let cargo_working_dir = nested_cargo_working_dir(workspace_dir);
    let target_dir = match env::var_os("CARGO_TARGET_DIR") {
        Some(path) if Path::new(&path).is_absolute() => PathBuf::from(path),
        Some(path) => workspace_dir.join(path),
        None => workspace_dir.join("target"),
    };

    println!("building the servokit-host-desktop static library");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let mut build = Command::new(&cargo);
    build
        .current_dir(&cargo_working_dir)
        .env("CARGO_TERM_COLOR", "never")
        .arg("rustc")
        .arg("--manifest-path")
        .arg(workspace_dir.join("Cargo.toml"))
        .arg("-p")
        .arg(PACKAGE)
        .arg("--locked")
        .arg("--lib");
    if profile == "release" {
        build.arg("--release");
    }
    build.arg("--").arg("--print").arg("native-static-libs");
    let build_output = run(&mut build, "build the Rust static library");
    let native_libraries = native_static_libraries(&build_output);

    let profile_dir = match env::var_os("CARGO_BUILD_TARGET") {
        Some(target) => target_dir.join(target).join(profile),
        None => target_dir.join(profile),
    };
    let static_library = profile_dir.join(STATIC_LIBRARY);
    assert!(
        static_library.is_file(),
        "Cargo did not produce {}",
        static_library.display()
    );

    let output_dir = target_dir.join("c-boundary");
    fs::create_dir_all(&output_dir).unwrap_or_else(|error| {
        panic!(
            "failed to create C boundary output directory {}: {error}",
            output_dir.display()
        )
    });

    let link_search = platform_link_search(
        &cargo,
        workspace_dir,
        &cargo_working_dir,
        &profile_dir,
        &output_dir,
        &native_libraries,
    );
    for (mode, compiler_flags) in [
        ("normal", &["-O0"][..]),
        ("ndebug", &["-O2", "-DNDEBUG"][..]),
    ] {
        let executable = output_dir.join(format!("c_boundary-{mode}{}", env::consts::EXE_SUFFIX));
        println!("compiling and linking the {mode} C++17 harness");

        let mut compiler = cxx_command();
        compiler
            .args(cxx_common_flags())
            .args(cxx_mode_flags(mode, compiler_flags))
            .arg("-I")
            .arg(crate_dir.join("include"))
            .arg(crate_dir.join("tests/c_boundary.cpp"))
            .arg(&static_library);
        finish_link(&mut compiler, &executable, &link_search, &native_libraries);
        run(&mut compiler, "compile and link the C++ boundary harness");

        println!("running the {mode} C++17 harness");
        run(
            &mut Command::new(&executable),
            "run the C++ boundary harness",
        );
    }

    native_lifecycle(
        &crate_dir,
        &output_dir,
        &static_library,
        &link_search,
        &native_libraries,
    );
}

#[cfg(target_os = "macos")]
fn nested_cargo_working_dir(workspace_dir: &Path) -> PathBuf {
    workspace_dir.to_path_buf()
}

#[cfg(target_os = "windows")]
fn nested_cargo_working_dir(_workspace_dir: &Path) -> PathBuf {
    env::temp_dir()
}

#[cfg(target_os = "macos")]
fn native_lifecycle(
    crate_dir: &Path,
    output_dir: &Path,
    static_library: &Path,
    link_search: &[OsString],
    native_libraries: &[OsString],
) {
    let bundle = output_dir.join("ServoKitNativeLifecycle.app");
    let contents = bundle.join("Contents");
    let executable_dir = contents.join("MacOS");
    fs::create_dir_all(&executable_dir).expect("create the AppKit test bundle");
    fs::write(
        contents.join("Info.plist"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>ServoKitNativeLifecycle</string>
<key>CFBundleIdentifier</key><string>org.servokit.native-lifecycle-test</string>
<key>CFBundleName</key><string>ServoKitNativeLifecycle</string>
<key>CFBundlePackageType</key><string>APPL</string>
</dict></plist>
"#,
    )
    .expect("write the AppKit test bundle plist");
    let executable = executable_dir.join("ServoKitNativeLifecycle");
    println!("compiling and linking the AppKit NSView lifecycle harness");
    let mut compiler = cxx_command();
    compiler
        .args(cxx_common_flags())
        .arg("-fobjc-arc")
        .arg("-I")
        .arg(crate_dir.join("include"))
        .arg(crate_dir.join("tests/macos_native_lifecycle.mm"))
        .arg(static_library);
    finish_link_with_extra(
        &mut compiler,
        &executable,
        link_search,
        native_libraries,
        &[OsString::from("-framework"), OsString::from("AppKit")],
    );
    run(
        &mut compiler,
        "compile and link the AppKit lifecycle harness",
    );

    if env::var_os("SERVOKIT_RUN_APPKIT_LIFECYCLE").is_none() {
        println!(
            "set SERVOKIT_RUN_APPKIT_LIFECYCLE=1 in an interactive macOS session to run the NSView lifecycle"
        );
        return;
    }

    let result = output_dir.join("macos-native-lifecycle-result.txt");
    let _ = fs::remove_file(&result);
    println!("launching the AppKit NSView lifecycle harness");
    let mut launch = Command::new("open");
    launch
        .arg("-W")
        .arg("-n")
        .arg(&bundle)
        .arg("--args")
        .arg(&result);
    run(&mut launch, "launch the AppKit lifecycle harness");
    let result = fs::read_to_string(&result).expect("read the AppKit lifecycle result");
    assert_eq!(
        result, "passed\n",
        "the AppKit lifecycle harness did not report success"
    );
}

#[cfg(target_os = "windows")]
fn native_lifecycle(
    crate_dir: &Path,
    output_dir: &Path,
    static_library: &Path,
    link_search: &[OsString],
    native_libraries: &[OsString],
) {
    let executable = output_dir.join(format!(
        "windows_native_lifecycle{}",
        env::consts::EXE_SUFFIX
    ));
    println!("compiling and linking the Win32 HWND lifecycle harness");
    let mut compiler = cxx_command();
    compiler
        .args(cxx_common_flags())
        .arg("-I")
        .arg(crate_dir.join("include"))
        .arg(crate_dir.join("tests/windows_native_lifecycle.cpp"))
        .arg(static_library);
    finish_link_with_extra(
        &mut compiler,
        &executable,
        link_search,
        native_libraries,
        &[OsString::from("user32.lib")],
    );
    run(
        &mut compiler,
        "compile and link the Win32 lifecycle harness",
    );

    println!("running the Win32 HWND lifecycle harness");
    run_with_timeout(
        &mut Command::new(&executable),
        "run the Win32 lifecycle harness",
        Duration::from_secs(60),
    );
}

#[cfg(target_os = "windows")]
fn run_with_timeout(command: &mut Command, action: &str, timeout: Duration) -> Output {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to {action}: {error}"));
    let started = Instant::now();
    while started.elapsed() < timeout {
        if child
            .try_wait()
            .unwrap_or_else(|error| panic!("failed to wait while attempting to {action}: {error}"))
            .is_some()
        {
            let output = child
                .wait_with_output()
                .unwrap_or_else(|error| panic!("failed to collect output after {action}: {error}"));
            assert!(
                output.status.success(),
                "failed to {action} ({})\nstdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return output;
        }
        thread::sleep(Duration::from_millis(50));
    }

    child
        .kill()
        .unwrap_or_else(|error| panic!("failed to stop timed-out {action}: {error}"));
    let output = child.wait_with_output().unwrap_or_else(|error| {
        panic!("failed to collect output after timed-out {action}: {error}")
    });
    panic!(
        "timed out after {}s while attempting to {action}\nstdout:\n{}\nstderr:\n{}",
        timeout.as_secs(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn native_static_libraries(output: &Output) -> Vec<OsString> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let libraries = stdout
        .lines()
        .chain(stderr.lines())
        .find_map(|line| line.split_once("native-static-libs:"))
        .map(|(_, libraries)| {
            libraries
                .split_ascii_whitespace()
                .map(OsString::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            panic!(
                "rustc did not report native static libraries\nstdout:\n{stdout}\nstderr:\n{stderr}"
            )
        });
    assert!(!libraries.is_empty(), "rustc reported no native libraries");
    libraries
}

fn run(command: &mut Command, action: &str) -> Output {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("failed to {action}: {error}"));
    assert!(
        output.status.success(),
        "failed to {action} ({})\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[cfg(target_os = "macos")]
fn cxx_command() -> Command {
    let mut command = Command::new("xcrun");
    command.arg("clang++");
    command
}

#[cfg(target_os = "windows")]
fn cxx_command() -> Command {
    Command::new(env::var_os("CXX").unwrap_or_else(|| OsString::from("clang-cl.exe")))
}

#[cfg(target_os = "macos")]
fn platform_link_search(
    _cargo: &OsStr,
    _workspace_dir: &Path,
    _cargo_working_dir: &Path,
    _profile_dir: &Path,
    _output_dir: &Path,
    _native_libraries: &[OsString],
) -> Vec<OsString> {
    let mut pkg_config = Command::new("pkg-config");
    pkg_config.arg("--libs-only-L").arg("freetype2");
    let output = run(&mut pkg_config, "query the FreeType link search path");
    String::from_utf8(output.stdout)
        .expect("pkg-config must return UTF-8 arguments")
        .split_ascii_whitespace()
        .map(OsString::from)
        .collect()
}

#[cfg(target_os = "windows")]
fn platform_link_search(
    cargo: &OsStr,
    workspace_dir: &Path,
    cargo_working_dir: &Path,
    profile_dir: &Path,
    output_dir: &Path,
    native_libraries: &[OsString],
) -> Vec<OsString> {
    let windows_import_library = native_libraries
        .iter()
        .find(|library| {
            let library = library.to_string_lossy();
            library.starts_with("windows.") && library.ends_with(".lib")
        })
        .unwrap_or_else(|| panic!("rustc did not report the Windows import library"));
    let package_name = match env::consts::ARCH {
        "aarch64" => "windows_aarch64_msvc",
        "x86" => "windows_i686_msvc",
        "x86_64" => "windows_x86_64_msvc",
        arch => panic!("unsupported Windows architecture for C boundary test: {arch}"),
    };

    let mut metadata = Command::new(cargo);
    metadata
        .current_dir(cargo_working_dir)
        .env("CARGO_TERM_COLOR", "never")
        .arg("metadata")
        .arg("--manifest-path")
        .arg(workspace_dir.join("Cargo.toml"))
        .arg("--locked")
        .arg("--format-version")
        .arg("1");
    let output = run(&mut metadata, "locate the Windows import library");
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Cargo metadata must be valid JSON");
    let library_dir = metadata["packages"]
        .as_array()
        .expect("Cargo metadata packages must be an array")
        .iter()
        .filter(|package| package["name"].as_str() == Some(package_name))
        .filter_map(|package| package["manifest_path"].as_str())
        .map(PathBuf::from)
        .filter_map(|manifest| manifest.parent().map(|directory| directory.join("lib")))
        .find(|directory| directory.join(windows_import_library).is_file())
        .unwrap_or_else(|| {
            panic!(
                "Cargo metadata did not locate {package_name}/lib/{}",
                windows_import_library.to_string_lossy()
            )
        });

    let mozangle_dir = fs::read_dir(profile_dir.join("build"))
        .expect("read Cargo build output directory")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("mozangle-"))
        .map(|entry| entry.path().join("out"))
        .find(|directory| {
            directory.join("libEGL.lib").is_file() && directory.join("libGLESv2.lib").is_file()
        })
        .expect("locate the MozANGLE import libraries");
    for dll in ["libEGL.dll", "libGLESv2.dll"] {
        fs::copy(mozangle_dir.join(dll), output_dir.join(dll))
            .unwrap_or_else(|error| panic!("stage {dll} for the C boundary harness: {error}"));
    }

    [library_dir, mozangle_dir]
        .into_iter()
        .map(|directory| OsString::from(format!("/LIBPATH:{}", directory.display())))
        .collect()
}

#[cfg(target_os = "macos")]
fn cxx_common_flags() -> &'static [&'static str] {
    &["-std=c++17", "-Wall", "-Wextra", "-Werror"]
}

#[cfg(target_os = "windows")]
fn cxx_common_flags() -> &'static [&'static str] {
    &["/std:c++17", "/EHsc", "/MD", "/W4", "/WX", "-fuse-ld=lld"]
}

#[cfg(target_os = "macos")]
fn cxx_mode_flags<'a>(_mode: &str, flags: &'a [&'a str]) -> &'a [&'a str] {
    flags
}

#[cfg(target_os = "windows")]
fn cxx_mode_flags<'a>(mode: &str, _flags: &'a [&'a str]) -> &'a [&'a str] {
    match mode {
        "normal" => &["/Od"],
        "ndebug" => &["/O2", "/DNDEBUG"],
        _ => panic!("unsupported C boundary compiler mode: {mode}"),
    }
}

fn finish_link(
    command: &mut Command,
    executable: &Path,
    link_search: &[OsString],
    native_libraries: &[OsString],
) {
    finish_link_with_extra(command, executable, link_search, native_libraries, &[]);
}

#[cfg(target_os = "macos")]
fn finish_link_with_extra(
    command: &mut Command,
    executable: &Path,
    link_search: &[OsString],
    native_libraries: &[OsString],
    extra_libraries: &[OsString],
) {
    command
        .args(link_search)
        .args(native_libraries)
        .args(extra_libraries)
        .arg("-Wl,-no_fixup_chains")
        .arg("-o")
        .arg(executable);
}

#[cfg(target_os = "windows")]
fn finish_link_with_extra(
    command: &mut Command,
    executable: &Path,
    link_search: &[OsString],
    native_libraries: &[OsString],
    extra_libraries: &[OsString],
) {
    command.arg(OsString::from(format!("/Fe{}", executable.display())));
    command.args(
        native_libraries
            .iter()
            .filter(|library| !library.to_string_lossy().starts_with('/')),
    );
    command.args(extra_libraries).arg("/link").args(link_search);
    command.args(
        native_libraries
            .iter()
            .filter(|library| library.to_string_lossy().starts_with('/')),
    );
}
