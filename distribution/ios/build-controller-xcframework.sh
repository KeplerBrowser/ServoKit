#!/bin/bash
set -euo pipefail
export LC_ALL=C IPHONEOS_DEPLOYMENT_TARGET=15.1

case "${BASH_SOURCE[0]}" in
  */*) script_dir="${BASH_SOURCE[0]%/*}" ;;
  *) script_dir=. ;;
esac
root="$(cd "${script_dir}/../.." && pwd -P)"
manifest="${root}/crates/Cargo.toml"
header="${root}/crates/servokit-controller-ffi/include/ServoKitController.h"
header_dir="${header%/*}"
target_dir="${root}/crates/target"
output="${root}/packages/react-native-servokit/ios/ServoKitController.xcframework"
lock="${output}.lock"
work="${lock}/work"
backup="${work}/previous.xcframework"
export CARGO_TARGET_DIR="${target_dir}"

fail() { echo "$*" >&2; exit 1; }
same() { [ "$1" = "$2" ] || fail "$3: expected '$2', got '$1'"; }
disk() {
  local available
  available="$(df -Pk "${root}" | awk 'NR == 2 { print $4 }')"
  [ "${available}" -ge 16777216 ] || fail "Disk floor reached: ${available} KiB available"
  echo "Disk available: ${available} KiB"
}
version_at_most() {
  awk -v actual="$1" -v ceiling="$2" 'BEGIN {
    if (actual !~ /^[0-9]+([.][0-9]+)*$/ || ceiling !~ /^[0-9]+([.][0-9]+)*$/) exit 2
    actual_parts = split(actual, a, ".")
    ceiling_parts = split(ceiling, c, ".")
    parts = actual_parts > ceiling_parts ? actual_parts : ceiling_parts
    for (i = 1; i <= parts; i++) {
      if ((a[i] + 0) < (c[i] + 0)) exit 0
      if ((a[i] + 0) > (c[i] + 0)) exit 1
    }
    exit 0
  }'
}

[ "$#" -eq 0 ] || fail "Output is fixed at ${output}; no arguments are accepted"
for tool in awk cargo cat cmp df file find grep mkdir mv plutil rm rustup sort stat tail tr \
  uniq wc xcodebuild xcrun; do
  command -v "${tool}" >/dev/null || fail "Missing required tool: ${tool}"
done
for tool in ar clang lipo llvm-nm nm otool vtool; do
  xcrun --find "${tool}" >/dev/null || fail "Missing required Xcode tool: ${tool}"
done
xcrun --sdk iphoneos --show-sdk-path >/dev/null || fail "Missing required SDK: iphoneos"
xcrun --sdk iphonesimulator --show-sdk-path >/dev/null || fail "Missing required SDK: iphonesimulator"
[ -f "${manifest}" ] || fail "Missing workspace manifest: ${manifest}"
[ -f "${header}" ] || fail "Missing controller header: ${header}"

lock_acquired=0
committed=0
cleanup() {
  local status=$?
  local retain=0
  trap - EXIT HUP INT TERM
  set +e
  if [ "${lock_acquired}" -eq 0 ]; then
    exit "${status}"
  fi
  if [ "${committed}" -eq 0 ] && { [ -e "${backup}" ] || [ -L "${backup}" ]; }; then
    if [ ! -e "${output}" ] && [ ! -L "${output}" ]; then
      if ! mv "${backup}" "${output}"; then
        echo "Failed to restore previous XCFramework; backup retained at ${backup}" >&2
        retain=1
      fi
    else
      echo "Uncommitted output path is occupied; backup retained at ${backup}" >&2
      retain=1
    fi
  fi
  if [ "${committed}" -eq 0 ] && [ "${status}" -eq 0 ]; then
    status=1
  fi
  if [ "${retain}" -eq 1 ]; then
    echo "Producer lock and recovery state retained at ${lock}" >&2
    status=1
  elif ! rm -rf "${lock}"; then
    echo "Failed to remove producer lock: ${lock}" >&2
    status=1
  fi
  exit "${status}"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
if ! mkdir "${lock}" 2>/dev/null; then
  trap - EXIT HUP INT TERM
  fail "Another producer or retained recovery state holds ${lock}"
fi
lock_acquired=1
mkdir "${work}"

version_at_most 14.0 15.1 || fail "Deployment-version comparison rejected 14.0 <= 15.1"
version_at_most 15.1 15.1 || fail "Deployment-version comparison rejected 15.1 <= 15.1"
if version_at_most 15.2 15.1; then
  fail "Deployment-version comparison accepted 15.2 > 15.1"
fi
echo "Deployment-version comparison self-check passed: 14.0, 15.1 <= 15.1; 15.2 rejected"

targets=(aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios)
installed="$(rustup target list --installed)"
for target in "${targets[@]}"; do
  grep -Fxq "${target}" <<<"${installed}" || fail "Missing Rust target: ${target}"
done

for target in "${targets[@]}"; do
  tree="$(cargo tree --manifest-path "${manifest}" --package servokit-controller-ffi \
    --locked --target "${target}" --edges normal --prefix none --format '{p}')"
  if grep -Eq '^servo([-_][^ ]*)? v' <<<"${tree}"; then
    fail "${target}: Servo-family package is present in the servokit-controller-ffi dependency tree"
  fi
done

printf '%s\n' _servokit_controller_create _servokit_controller_destroy \
  _servokit_controller_dispatch _servokit_controller_result_free > "${work}/expected-exports"
cat > "${work}/abi-probe.c" <<'EOF'
#include "ServoKitController.h"

#define FIELD_TYPE_IS(field, type) \
  __builtin_types_compatible_p(__typeof__(((ServoKitControllerResult *)0)->field), type)

_Static_assert(SERVOKIT_CONTROLLER_OK == 0, "wrong OK status");
_Static_assert(SERVOKIT_CONTROLLER_INVALID_ARGUMENT == 1, "wrong invalid-argument status");
_Static_assert(SERVOKIT_CONTROLLER_STALE_HANDLE == 2, "wrong stale-handle status");
_Static_assert(SERVOKIT_CONTROLLER_BUSY == 3, "wrong busy status");
_Static_assert(SERVOKIT_CONTROLLER_INTERNAL_ERROR == 4, "wrong internal-error status");
_Static_assert(SERVOKIT_CONTROLLER_PANIC == 5, "wrong panic status");
_Static_assert(sizeof(ServoKitControllerResult) == 32, "wrong result size");
_Static_assert(_Alignof(ServoKitControllerResult) == 8, "wrong result alignment");
_Static_assert(offsetof(ServoKitControllerResult, status) == 0, "wrong status offset");
_Static_assert(offsetof(ServoKitControllerResult, handle) == 8, "wrong handle offset");
_Static_assert(offsetof(ServoKitControllerResult, bytes) == 16, "wrong bytes offset");
_Static_assert(offsetof(ServoKitControllerResult, len) == 24, "wrong len offset");
_Static_assert(FIELD_TYPE_IS(status, uint32_t), "wrong status type");
_Static_assert(FIELD_TYPE_IS(handle, uint64_t), "wrong handle type");
_Static_assert(FIELD_TYPE_IS(bytes, const uint8_t *), "wrong bytes type");
_Static_assert(FIELD_TYPE_IS(len, size_t), "wrong len type");

static ServoKitControllerResult (*const create_fn)(void) = servokit_controller_create;
static ServoKitControllerResult (*const dispatch_fn)(uint64_t, const uint8_t *, size_t) =
    servokit_controller_dispatch;
static ServoKitControllerResult (*const destroy_fn)(uint64_t) = servokit_controller_destroy;
static void (*const result_free_fn)(ServoKitControllerResult) = servokit_controller_result_free;

int main(void) {
  return !(create_fn && dispatch_fn && destroy_fn && result_free_fn);
}
EOF

disk
for target in "${targets[@]}"; do
  echo "Building ${target}"
  cargo build --manifest-path "${manifest}" --package servokit-controller-ffi \
    --target "${target}" --release --locked
  disk
done

device="${target_dir}/aarch64-apple-ios/release/libservokit_controller_ffi.a"
sim_arm64="${target_dir}/aarch64-apple-ios-sim/release/libservokit_controller_ffi.a"
sim_x86_64="${target_dir}/x86_64-apple-ios/release/libservokit_controller_ffi.a"

inspect() {
  local archive="$1" tag="$2" arch="$3" sdk="$4" clang_target="$5" platform="$6"
  local probe version member object metadata kind build_count minos minos_count
  local metadata_count=0 bitcode_count=0 controller_count=0
  [ -f "${archive}" ] || fail "Missing release archive: ${archive}"
  same "$(xcrun lipo -archs "${archive}")" "${arch}" "${tag} architecture"

  mkdir -p "${work}/objects/${tag}"
  xcrun ar -t "${archive}" > "${work}/members-${tag}"
  grep -Ev '^__[.]SYMDEF' "${work}/members-${tag}" > "${work}/object-members-${tag}"
  same "$(wc -l < "${work}/object-members-${tag}" | tr -d ' ')" \
    "$(sort -u "${work}/object-members-${tag}" | wc -l | tr -d ' ')" \
    "${tag} archive member uniqueness"
  (
    cd "${work}/objects/${tag}"
    xcrun ar -x "${archive}"
  )
  : > "${work}/controller-exports-${tag}"
  while IFS= read -r member; do
    object="${work}/objects/${tag}/${member}"
    [ -f "${object}" ] || fail "${tag}: archive member was not extracted: ${member}"
    metadata="${object}.build-version"
    if ! xcrun vtool -show-build "${object}" > "${metadata}" 2> "${metadata}.error"; then
      kind="$(file -b "${object}")"
      case "${kind}" in
        *LLVM*bitcode*) bitcode_count=$((bitcode_count + 1)); continue ;;
        *) fail "${tag}: unreadable archive member ${member}: ${kind}" ;;
      esac
    fi
    if grep -Fq 'cmd LC_BUILD_VERSION' "${metadata}"; then
      build_count="$(grep -Fc 'cmd LC_BUILD_VERSION' "${metadata}")"
      metadata_count=$((metadata_count + 1))
      same "$(grep -Ec "^[[:space:]]*platform ${platform}$" "${metadata}")" \
        "${build_count}" "${tag}: ${member} platform metadata count"
      minos_count="$(awk '$1 == "minos" { count++ } END { print count + 0 }' "${metadata}")"
      same "${minos_count}" "${build_count}" "${tag}: ${member} deployment metadata count"
      while IFS= read -r minos; do
        version_at_most "${minos}" 15.1 ||
          fail "${tag}: ${member} requires deployment target ${minos}, newer than 15.1"
      done < <(awk '$1 == "minos" { print $2 }' "${metadata}")
    fi
    case "${member}" in
      servokit_controller_ffi-*)
        xcrun nm -gjU "${object}" > "${object}.exports"
        if grep -E '^_servokit_controller_[[:alnum:]_]+$' "${object}.exports" \
          >> "${work}/controller-exports-${tag}"; then
          grep -Fq 'cmd LC_BUILD_VERSION' "${metadata}" ||
            fail "${tag}: controller export object lacks readable LC_BUILD_VERSION metadata"
          same "$(grep -Ec '^[[:space:]]*minos 15[.]1$' "${metadata}")" \
            "${build_count}" "${tag}: controller export object deployment metadata count"
          controller_count=$((controller_count + 1))
        fi
        ;;
    esac
  done < "${work}/object-members-${tag}"
  [ "${metadata_count}" -gt 0 ] || fail "${tag}: no readable LC_BUILD_VERSION archive members"
  [ "${controller_count}" -gt 0 ] || fail "${tag}: controller export object not found"
  sort -u "${work}/controller-exports-${tag}" > "${work}/controller-exports-sorted-${tag}"
  cmp "${work}/expected-exports" "${work}/controller-exports-sorted-${tag}" ||
    fail "${tag}: controller object does not contain exactly the locked four exports"
  echo "${tag}: ${metadata_count} metadata-bearing members verified; ${bitcode_count} unreadable LLVM bitcode members"

  if ! xcrun llvm-nm --defined-only --extern-only --no-sort "${archive}" \
    > "${work}/llvm-nm-${tag}" 2> "${work}/llvm-nm-${tag}.error"; then
    grep -Fq 'Unknown attribute kind (105)' "${work}/llvm-nm-${tag}.error" ||
      fail "${tag}: llvm-nm archive scan failed for an unexpected reason"
    echo "${tag}: llvm-nm archive scan is incomplete; LLVM 22 bitcode reader failures recorded, force-link proof required"
  fi

  probe="${work}/probe-${tag}"
  xcrun --sdk "${sdk}" clang -target "${clang_target}" -std=c11 -Wall -Wextra -Werror \
    -I "${header_dir}" "${work}/abi-probe.c" \
    -Xlinker -force_load -Xlinker "${archive}" -Xlinker -fatal_warnings -o "${probe}"
  version="$(xcrun vtool -show-build "${probe}")"
  grep -Eq "^[[:space:]]*platform ${platform}$" <<<"${version}" || fail "${tag}: wrong platform"
  grep -Eq '^[[:space:]]*minos 15[.]1$' <<<"${version}" || fail "${tag}: wrong deployment target"
  xcrun nm -gjU "${probe}" > "${work}/all-exports-${tag}"
  { grep -E '^_servokit_controller_[[:alnum:]_]+$' "${work}/all-exports-${tag}" || true; } |
    sort -u > "${work}/exports-${tag}"
  cmp "${work}/expected-exports" "${work}/exports-${tag}" ||
    fail "${tag}: missing or extra servokit_controller_* exports"
  xcrun otool -L "${probe}" | tail -n +2 | awk '{ print $1 }' > "${work}/deps-${tag}"
  same "$(cat "${work}/deps-${tag}")" /usr/lib/libSystem.B.dylib "${tag} dependencies"
}

inspect "${device}" device arm64 iphoneos arm64-apple-ios15.1 IOS
inspect "${sim_arm64}" simulator-arm64 arm64 iphonesimulator \
  arm64-apple-ios15.1-simulator IOSSIMULATOR
inspect "${sim_x86_64}" simulator-x86_64 x86_64 iphonesimulator \
  x86_64-apple-ios15.1-simulator IOSSIMULATOR

mkdir -p "${work}/simulator"
simulator="${work}/simulator/libservokit_controller_ffi.a"
xcrun lipo -create "${sim_arm64}" "${sim_x86_64}" -output "${simulator}"
sim_archs="$(xcrun lipo -archs "${simulator}" | tr ' ' '\n' | sort | tr '\n' ' ')"
same "${sim_archs}" 'arm64 x86_64 ' "Universal simulator architectures"

staged="${work}/ServoKitController.xcframework"
xcodebuild -create-xcframework \
  -library "${device}" -headers "${header_dir}" \
  -library "${simulator}" -headers "${header_dir}" \
  -output "${staged}"

printf '%s\n' \
  ./Info.plist \
  ./ios-arm64 \
  ./ios-arm64/Headers \
  ./ios-arm64/Headers/ServoKitController.h \
  ./ios-arm64/libservokit_controller_ffi.a \
  ./ios-arm64_x86_64-simulator \
  ./ios-arm64_x86_64-simulator/Headers \
  ./ios-arm64_x86_64-simulator/Headers/ServoKitController.h \
  ./ios-arm64_x86_64-simulator/libservokit_controller_ffi.a | sort > "${work}/expected-entries"
if find "${staged}" -type l -print -quit | grep -q .; then
  fail "XCFramework contains a symlink"
fi
(
  cd "${staged}"
  find . -mindepth 1 -print | sort
) > "${work}/entries"
cmp "${work}/expected-entries" "${work}/entries" || fail "Wrong XCFramework entry set"
for directory in ios-arm64 ios-arm64/Headers ios-arm64_x86_64-simulator \
  ios-arm64_x86_64-simulator/Headers; do
  [ -d "${staged}/${directory}" ] || fail "XCFramework entry is not a directory: ${directory}"
done
for file_path in Info.plist ios-arm64/Headers/ServoKitController.h \
  ios-arm64/libservokit_controller_ffi.a \
  ios-arm64_x86_64-simulator/Headers/ServoKitController.h \
  ios-arm64_x86_64-simulator/libservokit_controller_ffi.a; do
  [ -f "${staged}/${file_path}" ] || fail "XCFramework entry is not a file: ${file_path}"
done

plist="${staged}/Info.plist"
device_plist='{"BinaryPath":"libservokit_controller_ffi.a","HeadersPath":"Headers","LibraryIdentifier":"ios-arm64","LibraryPath":"libservokit_controller_ffi.a","SupportedArchitectures":["arm64"],"SupportedPlatform":"ios"}'
simulator_plist='{"BinaryPath":"libservokit_controller_ffi.a","HeadersPath":"Headers","LibraryIdentifier":"ios-arm64_x86_64-simulator","LibraryPath":"libservokit_controller_ffi.a","SupportedArchitectures":["arm64","x86_64"],"SupportedPlatform":"ios","SupportedPlatformVariant":"simulator"}'
same "$(plutil -extract AvailableLibraries raw "${plist}")" 2 "XCFramework library count"
id0="$(plutil -extract AvailableLibraries.0.LibraryIdentifier raw "${plist}")"
id1="$(plutil -extract AvailableLibraries.1.LibraryIdentifier raw "${plist}")"
case "${id0}:${id1}" in
  ios-arm64:ios-arm64_x86_64-simulator) device_index=0; simulator_index=1 ;;
  ios-arm64_x86_64-simulator:ios-arm64) simulator_index=0; device_index=1 ;;
  *) fail "Wrong XCFramework library identifiers: ${id0}, ${id1}" ;;
esac
expected_plist="${work}/expected.plist"
plutil -create xml1 "${expected_plist}"
plutil -insert AvailableLibraries -json "[${device_plist},${simulator_plist}]" "${expected_plist}"
plutil -insert CFBundlePackageType -string XFWK "${expected_plist}"
plutil -insert XCFrameworkFormatVersion -string 1.0 "${expected_plist}"
plutil -convert xml1 -o "${work}/actual.plist" "${plist}"
device_actual="$(plutil -extract "AvailableLibraries.${device_index}" json -o - "${plist}")"
simulator_actual="$(plutil -extract "AvailableLibraries.${simulator_index}" json -o - "${plist}")"
plutil -replace AvailableLibraries -json "[${device_actual},${simulator_actual}]" "${work}/actual.plist"
plutil -convert xml1 "${expected_plist}"
plutil -convert xml1 "${work}/actual.plist"
cmp "${expected_plist}" "${work}/actual.plist" || fail "Wrong XCFramework plist schema or content"

packaged_device="${staged}/ios-arm64/libservokit_controller_ffi.a"
packaged_simulator="${staged}/ios-arm64_x86_64-simulator/libservokit_controller_ffi.a"
cmp "${device}" "${packaged_device}" || fail "Packaged device archive differs from release archive"
cmp "${simulator}" "${packaged_simulator}" || fail "Packaged simulator archive differs from lipo output"
for slice in ios-arm64 ios-arm64_x86_64-simulator; do
  packaged_header="${staged}/${slice}/Headers/ServoKitController.h"
  cmp "${header}" "${packaged_header}" || fail "${slice}: wrong packaged header"
done

staged_identity="$(stat -f '%d:%i' "${staged}")"
if [ -e "${output}" ] || [ -L "${output}" ]; then
  mv "${output}" "${backup}"
fi
mv "${staged}" "${output}"
same "$(stat -f '%d:%i' "${output}")" "${staged_identity}" "Installed XCFramework identity"
committed=1
echo "Created ${output}"
