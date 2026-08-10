#![cfg(any(target_os = "macos", target_os = "windows"))]

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PACKAGE: &str = "servokit-host-desktop";

#[cfg(target_os = "macos")]
const STATIC_LIBRARY: &str = "libservokit_host_desktop.a";
#[cfg(all(target_os = "windows", target_env = "msvc"))]
const STATIC_LIBRARY: &str = "servokit_host_desktop.lib";
#[cfg(all(target_os = "windows", target_env = "gnu"))]
const STATIC_LIBRARY: &str = "libservokit_host_desktop.a";

#[test]
fn c_boundary() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = crate_dir.parent().expect("crate must be in the workspace");
    let target_dir = match env::var_os("CARGO_TARGET_DIR") {
        Some(path) if Path::new(&path).is_absolute() => PathBuf::from(path),
        Some(path) => workspace_dir.join(path),
        None => workspace_dir.join("target"),
    };

    println!("building the servokit-host-desktop static library");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let mut build = Command::new(cargo);
    build
        .current_dir(workspace_dir)
        .arg("rustc")
        .arg("--manifest-path")
        .arg(workspace_dir.join("Cargo.toml"))
        .arg("-p")
        .arg(PACKAGE)
        .arg("--locked")
        .arg("--lib")
        .arg("--")
        .arg("--print")
        .arg("native-static-libs");
    let build_output = run(&mut build, "build the Rust static library");
    let native_libraries = native_static_libraries(&build_output);

    let profile_dir = match env::var_os("CARGO_BUILD_TARGET") {
        Some(target) => target_dir.join(target).join("debug"),
        None => target_dir.join("debug"),
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

    let link_search = platform_link_search();
    for (mode, compiler_flags) in [
        ("normal", &["-O0"][..]),
        ("ndebug", &["-O2", "-DNDEBUG"][..]),
    ] {
        let executable = output_dir.join(format!("c_boundary-{mode}{}", env::consts::EXE_SUFFIX));
        println!("compiling and linking the {mode} C++17 harness");

        let mut compiler = cxx_command();
        compiler
            .arg("-std=c++17")
            .arg("-Wall")
            .arg("-Wextra")
            .arg("-Werror")
            .args(compiler_flags)
            .arg("-I")
            .arg(crate_dir.join("include"))
            .arg(crate_dir.join("tests/c_boundary.cpp"))
            .arg(&static_library)
            .args(&link_search)
            .args(&native_libraries)
            .args(platform_linker_flags())
            .arg("-o")
            .arg(&executable);
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
        .arg("-std=c++17")
        .arg("-fobjc-arc")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-I")
        .arg(crate_dir.join("include"))
        .arg(crate_dir.join("tests/macos_native_lifecycle.mm"))
        .arg(static_library)
        .args(link_search)
        .args(native_libraries)
        .args(platform_linker_flags())
        .arg("-framework")
        .arg("AppKit")
        .arg("-o")
        .arg(&executable);
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
        .arg("-std=c++17")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-I")
        .arg(crate_dir.join("include"))
        .arg(crate_dir.join("tests/windows_native_lifecycle.cpp"))
        .arg(static_library)
        .args(link_search)
        .args(native_libraries)
        .args(platform_linker_flags())
        .arg("-luser32")
        .arg("-o")
        .arg(&executable);
    run(
        &mut compiler,
        "compile and link the Win32 lifecycle harness",
    );

    println!("running the Win32 HWND lifecycle harness");
    run(
        &mut Command::new(&executable),
        "run the Win32 lifecycle harness",
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
    Command::new("clang++")
}

#[cfg(target_os = "macos")]
fn platform_link_search() -> Vec<OsString> {
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
fn platform_link_search() -> Vec<OsString> {
    Vec::new()
}

#[cfg(target_os = "macos")]
fn platform_linker_flags() -> &'static [&'static str] {
    &["-Wl,-no_fixup_chains"]
}

#[cfg(target_os = "windows")]
fn platform_linker_flags() -> &'static [&'static str] {
    &[]
}
