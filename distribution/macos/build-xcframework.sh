#!/bin/bash
set -euo pipefail
export LC_ALL=C
export TZ=UTC

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-/private/tmp/servokit-macos-cargo-target}"
cargo_home="${CARGO_HOME:-${HOME}/.cargo}"

reject_path_whitespace() {
  if [[ "$2" =~ [[:space:]] ]]; then
    echo "$1 contains unsupported whitespace" >&2
    exit 1
  fi
}
reject_path_whitespace HOME "${HOME}"
reject_path_whitespace CARGO_HOME "${cargo_home}"
reject_path_whitespace root "${root}"
reject_path_whitespace CARGO_TARGET_DIR "${target_dir}"
mkdir -p -- "${target_dir}"
target_dir="$(cd -- "${target_dir}" && pwd -P)"
reject_path_whitespace "physical CARGO_TARGET_DIR" "${target_dir}"
export CARGO_TARGET_DIR="${target_dir}"

package_json="${root}/packages/react-native-servokit/package.json"
header="${root}/crates/servokit-host-desktop/include/servokit_desktop_private.h"
package_version="$(bun -e 'console.log(require(process.argv[1]).version)' "${package_json}")"
version="${SERVOKIT_BINARY_VERSION:-${package_version}}"
output="${SERVOKIT_ARTIFACT_DIR:-/private/tmp/servokit-macos-${version}}"
work="${output}/work"
stage="${output}/stage"
evidence="${output}/evidence"
framework="${stage}/ServoKit.framework"
binary="${framework}/Versions/A/ServoKit"
dsym="${stage}/ServoKit.framework.dSYM"
xcframework="${stage}/ServoKit.xcframework"
metadata="${work}/cargo-metadata.json"
exports="${work}/ServoKit.exports"
release_root="${work}/release"
release_zip="${output}/ServoKit-macos-${version}.zip"
dsym_zip="${output}/ServoKit-macos-${version}-dSYM.zip"
source_url="${SERVOKIT_BINARY_SOURCE_URL:-https://github.com/KeplerBrowser/ServoKit/releases/download/v${version}/ServoKit-macos-${version}.zip}"
created="${SOURCE_DATE_EPOCH:-$(git -C "${root}" show -s --format=%ct HEAD)}"
created_iso="$(date -u -r "${created}" '+%Y-%m-%dT%H:%M:%SZ')"
archive_timestamp="$(date -u -r "${created}" '+%Y%m%d%H%M.%S')"
install_name="@rpath/ServoKit.framework/Versions/A/ServoKit"
link_libraries=(
  -framework AppKit
  -framework QuartzCore
  -framework Foundation
  -framework CoreGraphics
  -framework Security
  -lc++
  -lz
  -framework CoreText
  -framework CoreFoundation
  -framework OpenGL
  -framework CoreVideo
  -framework IOSurface
  -framework Metal
  -lobjc
  -liconv
)

if [ "${version}" != "${package_version}" ]; then
  echo "SERVOKIT_BINARY_VERSION ${version} does not match package version ${package_version}" >&2
  exit 1
fi

case "${output}" in
  /private/tmp/* | "${root}"/artifacts/* | */.servokit-source) ;;
  *) echo "SERVOKIT_ARTIFACT_DIR must be under /private/tmp, ${root}/artifacts, or end in /.servokit-source" >&2; exit 1 ;;
esac

if [ -e "${work}" ] || [ -e "${stage}" ] || [ -e "${evidence}" ]; then
  echo "SERVOKIT_ARTIFACT_DIR must not contain an existing build" >&2
  exit 1
fi
mkdir -p "${work}" "${stage}" "${evidence}" "${release_root}" "${framework}/Versions/A/Headers" \
  "${framework}/Versions/A/Modules" "${framework}/Versions/A/Resources"
: > "${evidence}/mozjs-object-build-versions.txt"

grep -Eo 'servokit_desktop_private_[a-z_]+' "${header}" | LC_ALL=C sort -u |
  sed 's/^/_/' > "${exports}"
if [ "$(wc -l < "${exports}" | tr -d ' ')" != "9" ]; then
  echo "Expected exactly nine private exports" >&2
  exit 1
fi

cargo metadata --manifest-path "${root}/crates/Cargo.toml" --locked \
  --format-version 1 --filter-platform aarch64-apple-darwin > "${metadata}"

export CARGO_PROFILE_RELEASE_DEBUG=1
export FREETYPE2_NO_PKG_CONFIG=1
export MACOSX_DEPLOYMENT_TARGET=14.0
export MOZJS_FROM_SOURCE=1
export SOURCE_DATE_EPOCH="${created}"
export ZERO_AR_DATE=1
rust_remap_flags=(
  "--remap-path-prefix=${HOME}=/home"
  "--remap-path-prefix=${cargo_home}=/cargo"
  "--remap-path-prefix=${root}=/src/servokit"
  "--remap-path-prefix=${target_dir}=/target"
)
printf -v CARGO_ENCODED_RUSTFLAGS '%s\x1f' "${rust_remap_flags[@]}"
CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS%$'\x1f'}"
export CARGO_ENCODED_RUSTFLAGS

clang_remap_flags=(
  "-ffile-prefix-map=${HOME}=/home"
  "-ffile-prefix-map=${cargo_home}=/cargo"
  "-ffile-prefix-map=${root}=/src/servokit"
  "-ffile-prefix-map=${target_dir}=/target"
)
export CFLAGS="${CFLAGS:+${CFLAGS} }-mmacosx-version-min=14.0 ${clang_remap_flags[*]}"
# mozjs_sys appends CXXFLAGS directly to "-stdlib=libc++"; keep the leading separator.
export CXXFLAGS=" ${clang_remap_flags[*]}"
export CC_x86_64_apple_darwin="$(xcrun --find clang) --target=x86_64-apple-darwin"
export CXX_x86_64_apple_darwin="$(xcrun --find clang++) --target=x86_64-apple-darwin"
export AR_x86_64_apple_darwin="$(xcrun --find ar)"

for target in aarch64-apple-darwin x86_64-apple-darwin; do
  cargo rustc \
    --manifest-path "${root}/crates/Cargo.toml" \
    --package servokit-host-desktop \
    --target "${target}" \
    --release \
    --locked

  case "${target}" in
    aarch64-apple-darwin) clang_target="arm64-apple-macos14.0" ;;
    x86_64-apple-darwin) clang_target="x86_64-apple-macos14.0" ;;
  esac

  archive="${target_dir}/${target}/release/libservokit_host_desktop.a"
  for object_name in jsapi jsglue; do
    members="$(xcrun ar -t "${archive}" | grep -E "(^|-)${object_name}[.]o$" || true)"
    member_count="$(printf '%s\n' "${members}" | sed '/^$/d' | wc -l | tr -d ' ')"
    if [ "${member_count}" != "1" ]; then
      echo "Expected one ${object_name}.o member in ${archive}, got ${member_count}" >&2
      exit 1
    fi
    member="${members}"
    object_dir="${work}/objects/${target}/${object_name}"
    mkdir -p "${object_dir}"
    (
      cd "${object_dir}"
      xcrun ar -x "${archive}" "${member}"
    )
    object_build_version="${object_dir}/${member}.build-version"
    xcrun vtool -show-build "${object_dir}/${member}" > "${object_build_version}"
    grep -Fq "platform MACOS" "${object_build_version}"
    grep -Fq "minos 14.0" "${object_build_version}"
    {
      printf '%s %s\n' "${target}" "${member}"
      cat "${object_build_version}"
    } >> "${evidence}/mozjs-object-build-versions.txt"
  done

  xcrun clang \
    -dynamiclib \
    -target "${clang_target}" \
    -Wl,-force_load,"${archive}" \
    -Wl,-exported_symbols_list,"${exports}" \
    -Wl,-install_name,"${install_name}" \
    -Wl,-dead_strip \
    -Wl,-fatal_warnings \
    "${link_libraries[@]}" \
    -o "${work}/ServoKit-${target}"
done

xcrun lipo -create \
  "${work}/ServoKit-aarch64-apple-darwin" \
  "${work}/ServoKit-x86_64-apple-darwin" \
  -output "${binary}"

architectures="$(xcrun lipo -archs "${binary}")"
if [ "$(wc -w <<<"${architectures}" | tr -d ' ')" != "2" ]; then
  echo "Expected exactly two framework architectures, got: ${architectures}" >&2
  exit 1
fi
for architecture in arm64 x86_64; do
  if [[ " ${architectures} " != *" ${architecture} "* ]]; then
    echo "Missing framework architecture: ${architecture}" >&2
    exit 1
  fi
done

xcrun nm -gjU "${binary}" | sort -u > "${evidence}/exports.txt"
if ! cmp -s "${exports}" "${evidence}/exports.txt"; then
  diff -u "${exports}" "${evidence}/exports.txt" >&2
  exit 1
fi

if ! xcrun otool -D "${binary}" | tail -n +2 | grep -Fxq "${install_name}"; then
  echo "Unexpected framework install name" >&2
  xcrun otool -D "${binary}" >&2
  exit 1
fi

for architecture in arm64 x86_64; do
  xcrun otool -L -arch "${architecture}" "${binary}" | tail -n +2 | awk '{print $1}'
done | sort -u > "${evidence}/dependencies.txt"
while IFS= read -r dependency; do
  case "${dependency}" in
    "${install_name}" | /System/Library/Frameworks/* | /usr/lib/*) ;;
    *) echo "Unadmitted dynamic dependency: ${dependency}" >&2; exit 1 ;;
  esac
done < "${evidence}/dependencies.txt"

: > "${evidence}/build-versions.txt"
for architecture in arm64 x86_64; do
  build_version="${work}/ServoKit.${architecture}.build-version"
  xcrun vtool -show-build -arch "${architecture}" "${binary}" > "${build_version}"
  grep -Fq "platform MACOS" "${build_version}"
  grep -Fq "minos 14.0" "${build_version}"
  cat "${build_version}" >> "${evidence}/build-versions.txt"
done

cp "${header}" "${framework}/Versions/A/Headers/servokit_desktop_private.h"

cat > "${framework}/Versions/A/Modules/module.modulemap" <<'EOF'
framework module ServoKit {
  umbrella header "servokit_desktop_private.h"
  export *
  module * { export * }
}
EOF

cat > "${framework}/Versions/A/Resources/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleExecutable</key><string>ServoKit</string>
  <key>CFBundleIdentifier</key><string>org.servo.servokit.runtime</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>ServoKit</string>
  <key>CFBundlePackageType</key><string>FMWK</string>
  <key>CFBundleShortVersionString</key><string>${version}</string>
  <key>CFBundleSupportedPlatforms</key><array><string>MacOSX</string></array>
  <key>CFBundleVersion</key><string>${version}</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
</dict>
</plist>
EOF

ln -s A "${framework}/Versions/Current"
ln -s Versions/Current/ServoKit "${framework}/ServoKit"
ln -s Versions/Current/Headers "${framework}/Headers"
ln -s Versions/Current/Modules "${framework}/Modules"
ln -s Versions/Current/Resources "${framework}/Resources"

xcrun dsymutil "${binary}" -o "${dsym}"
xcrun strip -S -x "${binary}"

for local_path in "${root}" "${target_dir}" "${cargo_home}" "${HOME}"; do
  if grep -aFq "${local_path}" "${binary}"; then
    echo "Stripped framework contains local path: ${local_path}" >&2
    exit 1
  fi
done

signing_identity="${SERVOKIT_CODESIGN_IDENTITY:--}"
if [ "${signing_identity}" = "-" ]; then
  codesign --force --sign - "${framework}"
  signing_status="ad-hoc"
else
  codesign --force --timestamp --options runtime \
    --sign "${signing_identity}" "${framework}"
  signing_status="signed"
fi
codesign --verify --strict --verbose=2 "${framework}"

binary_uuids="${work}/ServoKit.binary-uuids"
dsym_uuids="${work}/ServoKit.dsym-uuids"
xcrun dwarfdump --uuid "${binary}" | awk '{print $2, $3}' | LC_ALL=C sort > "${binary_uuids}"
xcrun dwarfdump --uuid "${dsym}" | awk '{print $2, $3}' | LC_ALL=C sort > "${dsym_uuids}"
cmp "${binary_uuids}" "${dsym_uuids}"
cp "${binary_uuids}" "${evidence}/binary-uuids.txt"
cp "${dsym_uuids}" "${evidence}/dsym-uuids.txt"

native_lifecycle="${work}/macos-native-lifecycle"
xcrun clang++ -std=c++17 -fobjc-arc \
  -I "${framework}/Headers" \
  -F "${stage}" \
  "${root}/crates/servokit-host-desktop/tests/macos_native_lifecycle.mm" \
  -framework ServoKit \
  -framework AppKit \
  -o "${native_lifecycle}"
DYLD_FRAMEWORK_PATH="${stage}" "${native_lifecycle}" "${work}/macos-native-lifecycle.result"
grep -Fxq "passed" "${work}/macos-native-lifecycle.result"
cp "${work}/macos-native-lifecycle.result" "${evidence}/native-lifecycle.txt"

xcodebuild -create-xcframework -framework "${framework}" -output "${xcframework}"
if [ "${signing_identity}" = "-" ]; then
  codesign --force --sign - "${xcframework}"
else
  codesign --force --timestamp --sign "${SERVOKIT_CODESIGN_IDENTITY}" "${xcframework}"
fi
codesign --verify --strict --verbose=2 "${xcframework}"

xcframework_plist="${xcframework}/Info.plist"
if [ "$(/usr/libexec/PlistBuddy -c 'Print :AvailableLibraries:0:SupportedPlatform' "${xcframework_plist}")" != "macos" ]; then
  echo "XCFramework contains a non-macOS platform" >&2
  exit 1
fi
if [ "$(/usr/libexec/PlistBuddy -c 'Print :AvailableLibraries' "${xcframework_plist}" | grep -c '^    Dict {$')" != "1" ]; then
  echo "XCFramework must contain one universal macOS library" >&2
  exit 1
fi

if [ "${SERVOKIT_FRAMEWORK_ONLY:-0}" = "1" ]; then
  printf 'Framework: %s\n' "${xcframework}"
  exit 0
fi

cp -R "${xcframework}" "${release_root}/ServoKit.xcframework"
mkdir -p "${release_root}/LICENSES"
cp "${root}/LICENSE" "${release_root}/LICENSES/ServoKit.txt"
bun "${root}/distribution/macos/create-sbom.mjs" \
  "${metadata}" \
  "${binary}" \
  "${release_root}/ServoKit.spdx.json" \
  "${version}" \
  "${created_iso}" \
  "${release_root}/LICENSES/Cargo" \
  "${release_root}/THIRD-PARTY-NOTICES.md"

if [ -e "${release_zip}" ] || [ -e "${dsym_zip}" ]; then
  echo "Refusing to overwrite existing release archives" >&2
  exit 1
fi
while IFS= read -r -d '' path; do
  touch -h -t "${archive_timestamp}" "${path}"
done < <(find "${release_root}" "${dsym}" -print0)
(
  cd "${release_root}"
  find . -print | LC_ALL=C sort | zip -X -q -y "${release_zip}" -@
)
(
  cd "${stage}"
  find ServoKit.framework.dSYM -print | LC_ALL=C sort | zip -X -q -y "${dsym_zip}" -@
)

release_sha256="$(shasum -a 256 "${release_zip}" | awk '{print $1}')"
dsym_sha256="$(shasum -a 256 "${dsym_zip}" | awk '{print $1}')"
header_sha256="$(shasum -a 256 "${header}" | awk '{print $1}')"
binary_sha256="$(shasum -a 256 "${binary}" | awk '{print $1}')"
binary_size="$(stat -f '%z' "${binary}")"
release_size="$(stat -f '%z' "${release_zip}")"
dsym_size="$(stat -f '%z' "${dsym_zip}")"
sed \
  -e "s/__VERSION__/${version}/g" \
  -e "s@__SOURCE_URL__@${source_url}@g" \
  -e "s/__SHA256__/${release_sha256}/g" \
  "${root}/distribution/macos/ServoKitMacOSBinary.podspec.template" \
  > "${output}/ServoKitMacOSBinary.podspec"
if grep -Eq '__[A-Z0-9_]+__' "${output}/ServoKitMacOSBinary.podspec"; then
  echo "Generated binary podspec contains an unresolved placeholder" >&2
  exit 1
fi
cp "${release_root}/ServoKit.spdx.json" "${output}/ServoKit.spdx.json"

cat > "${output}/manifest.json" <<EOF
{
  "version": "${version}",
  "architectures": ["arm64", "x86_64"],
  "minimumMacOSVersion": "14.0",
  "installName": "${install_name}",
  "privateHeaderSHA256": "${header_sha256}",
  "binarySHA256": "${binary_sha256}",
  "binarySize": ${binary_size},
  "releaseSHA256": "${release_sha256}",
  "releaseSize": ${release_size},
  "dSYMReleaseSHA256": "${dsym_sha256}",
  "dSYMReleaseSize": ${dsym_size},
  "signing": "${signing_status}",
  "mozjsBuild": "MOZJS_FROM_SOURCE=1",
  "nativeLifecycleArchitecture": "$(uname -m)",
  "xcode": "$(xcodebuild -version | paste -sd ' ' -)",
  "rustc": "$(rustc --version)",
  "sourceRevision": "$(git -C "${root}" rev-parse HEAD)"
}
EOF

(
  cd "${output}"
  shasum -a 256 \
    "ServoKit-macos-${version}.zip" \
    "ServoKit-macos-${version}-dSYM.zip" \
    "ServoKit.spdx.json" \
    "ServoKitMacOSBinary.podspec" > SHA256SUMS
)

(
  cd "${evidence}"
  shasum -a 256 ./*.txt > SHA256SUMS
)

printf 'Artifacts: %s\n' "${output}"
