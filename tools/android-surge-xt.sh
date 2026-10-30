#!/usr/bin/env bash
# Build Surge XT's *original* C++ DSP for Honeydew Android. No DSP recreation.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to the installed Android NDK}"
command -v cmake >/dev/null
command -v git >/dev/null
command -v ninja >/dev/null

ROOT="$PWD"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
mkdir -p "$TARGET"
TARGET="$(cd "$TARGET" && pwd)"
SRC="$TARGET/surge-xt-source"
BUILD="$TARGET/surge-xt-android"
JNI="$TARGET/android/jniLibs/arm64-v8a"
REV=5a4828a33ac4b2e2b45671b34e9e0e3f0c614765

if [ ! -d "$SRC/.git" ]; then
  git clone --filter=blob:none https://github.com/surge-synthesizer/surge.git "$SRC"
fi
# GitHub occasionally rejects fetching an otherwise valid, advertised HEAD by
# raw SHA ("upload-pack: not our ref"). Fetch its public branch ref instead,
# then verify and check out the exact pinned Surge XT revision.
git -C "$SRC" fetch --depth=1 origin main
if ! git -C "$SRC" cat-file -e "$REV^{commit}" 2>/dev/null; then
  git -C "$SRC" fetch --deepen=120 origin main
fi
git -C "$SRC" cat-file -e "$REV^{commit}"
git -C "$SRC" checkout --detach "$REV"
git -C "$SRC" submodule update --init --recursive --jobs 4

# Android Bionic has no glibc execinfo backtrace()/backtrace_symbols().
# This is sst-plugininfra's optional human-readable diagnostics routine;
# retaining the original Surge synthesis source and replacing only the
# unavailable diagnostic makes the core compatible with Android NDK.
python3 - "$SRC/libs/sst/sst-plugininfra/src/misc_linux.cpp" <<'PY'
from pathlib import Path
import sys
p = Path(sys.argv[1])
s = p.read_text()
old = '''std::string stackTraceToString(int depth)
{
    void *callstack[128];'''
new = '''std::string stackTraceToString(int depth)
{
#if defined(__ANDROID__)
    (void)depth;
    return "Native stack trace unavailable with Android Bionic";
#else
    void *callstack[128];'''
if old not in s:
    raise SystemExit("Expected pinned sst-plugininfra stack trace source changed")
s = s.replace(old, new, 1)
old_end = '''    free(strs);
    return oss.str();
}'''
new_end = '''    free(strs);
    return oss.str();
#endif
}'''
if old_end not in s:
    raise SystemExit("Expected pinned stack trace function ending changed")
p.write_text(s.replace(old_end, new_end, 1))
PY

# Surge's Linux CMake target links -lpthread, but Android Bionic provides
# pthreads directly in libc (there is no separate libpthread to link).
# Preserve SIMD and all other upstream Surge core dependencies.
python3 - "$SRC/src/common/CMakeLists.txt" <<'PY'
from pathlib import Path
import sys
p = Path(sys.argv[1])
s = p.read_text()
old = '      pthread\n'
new = '      $<$<NOT:$<PLATFORM_ID:Android>>:pthread>\n'
if s.count(old) != 2:
    raise SystemExit("Pinned Surge common CMake pthread branches changed")
p.write_text(s.replace(old, new))
PY

# The upstream files remain a verified Git checkout. Only its *build file*
# gets a local include for the Honeydew-specific linkage target.
python3 - "$SRC/CMakeLists.txt" "$ROOT/native/surge-xt/bridge.cmake" <<'PY'
from pathlib import Path
import sys
cmake = Path(sys.argv[1])
addon = f'\ninclude("{Path(sys.argv[2]).as_posix()}")\n'
text = cmake.read_text()
if addon not in text:
    cmake.write_text(text + addon)
PY

# Surge's upstream LuaJIT CMake helper runs make using the Linux HOST
# compiler. Cross-build LuaJIT with the Android NDK instead, including its
# host-side buildvm generator and real AArch64 target VM, and place the result
# where the original luajitlib CMake target looks for a prebuilt archive.
# This keeps Formula LFO and Lua-powered wavetable synthesis enabled.
TOOLBIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
LUA_CC="$TOOLBIN/aarch64-linux-android26-clang"
LUA_AR="$TOOLBIN/llvm-ar"
LUA_SRC="$SRC/libs/luajitlib/LuaJIT/src"
LUA_DST="$BUILD/libs/luajitlib/LuaJIT"
test -x "$LUA_CC"
test -x "$LUA_AR"
make -C "$LUA_SRC" clean
make -C "$LUA_SRC" --jobs "${SURGE_BUILD_JOBS:-3}" amalg \
  BUILDMODE=static HOST_CC=cc TARGET_SYS=Linux \
  TARGET_CC="$LUA_CC" TARGET_STCC="$LUA_CC" TARGET_LD="$LUA_CC" \
  TARGET_AR="$LUA_AR rcus" TARGET_STRIP="$TOOLBIN/llvm-strip" \
  TARGET_CFLAGS="-fPIC -fvisibility=hidden"
test -s "$LUA_SRC/libluajit.a"
mkdir -p "$LUA_DST/bin" "$LUA_DST/include"
cp "$LUA_SRC/libluajit.a" "$LUA_DST/bin/libluajit.a"
cp "$LUA_SRC/"*.h "$LUA_DST/include/"
cp "$LUA_SRC/"*.hpp "$LUA_DST/include/"
# Embed Surge XT's original factory Init Saw patch as native data.
# This avoids silently running an uninitialized synth on Android and
# makes the actual official FXP available without external storage rights.
mkdir -p "$BUILD/generated"
python3 - "$SRC/resources/data/patches_factory/Templates/Init Saw.fxp" "$BUILD/generated/honeydew_surge_init_patch.h" <<'PY'
from pathlib import Path
import sys
b = Path(sys.argv[1]).read_bytes()
if len(b) < 61 or b[:4] != b"CcnK" or b[8:12] != b"FPCh" or b[16:20] != b"cjs3":
    raise SystemExit("Original Surge Init Saw FXP header is not valid")
n = int.from_bytes(b[56:60], "big")
if n < 100 or 60 + n > len(b):
    raise SystemExit("Original Surge Init Saw FXP chunk size is invalid")
b = b[60:60+n]
text = "#pragma once\nstatic const unsigned char honeydew_surge_init_data[] = {\n"
for offset in range(0, len(b), 16):
    text += "    " + ", ".join(f"0x{v:02x}" for v in b[offset:offset+16]) + ",\n"
text += "};\n"
Path(sys.argv[2]).write_text(text)
PY

# Package every upstream factory FXP as an actual native binary bank (not
# approximate synthesis or parameter-only imitations). They are compiled as
# one .rodata blob via .incbin: 641 patches without a 100 MB C++ literal array.
python3 - "$SRC/resources/data/patches_factory" "$BUILD/generated" "$ROOT/src/content/presets/surge_xt_factory.tsv" <<'PY'
from pathlib import Path
import sys
root, out, manifest = (Path(v) for v in sys.argv[1:])
files = manifest.read_text().splitlines()
actual = sorted(p.relative_to(root).as_posix() for p in root.rglob("*.fxp"))
if actual != files or len(files) != 641:
    raise SystemExit("Pinned official Surge XT factory preset manifest does not match")
blob = bytearray()
records = []
for rel in files:
    f = (root / rel).read_bytes()
    if len(f) < 61 or f[:4] != b"CcnK" or f[8:12] != b"FPCh" or f[16:20] != b"cjs3":
        raise SystemExit("Invalid official Surge FXP: " + rel)
    size = int.from_bytes(f[56:60], "big")
    if size < 100 or 60 + size > len(f):
        raise SystemExit("Invalid original Surge FXP chunk: " + rel)
    records.append((len(blob), size))
    blob.extend(f[60:60+size])
(out / "surge_factory_patches.bin").write_bytes(blob)
arr = ",\n".join(f"    {{{offset}u, {size}u}}" for offset, size in records)
asmfile = (out / "surge_factory_patches.bin").as_posix()
head = f'''#pragma once
#include <cstdint>
struct HoneydewSurgeFactoryRecord {{ uint32_t offset; uint32_t size; }};
static constexpr int honeydew_surge_factory_count = {len(records)};
static constexpr HoneydewSurgeFactoryRecord honeydew_surge_factory[] = {{
{arr}
}};
extern "C" const unsigned char honeydew_surge_factory_blob_start[];
asm(R"surge(
.pushsection .rodata
.balign 16
.global honeydew_surge_factory_blob_start
honeydew_surge_factory_blob_start:
.incbin "{asmfile}"
.popsection
)surge");
'''
(out / "honeydew_surge_factory_bank.h").write_text(head)
print(f"Authentic Surge XT factory FXP bank: {len(files)} patches, {len(blob)} bytes")
PY

# Verify actual object architecture before letting Surge link the library.
"$TOOLBIN/llvm-ar" p "$LUA_DST/bin/libluajit.a" lj_vm.o > "$LUA_DST/lj_vm-check.o"
"$TOOLBIN/llvm-readelf" -h "$LUA_DST/lj_vm-check.o" | grep -F 'AArch64'
rm "$LUA_DST/lj_vm-check.o"

cmake -S "$SRC" -B "$BUILD" -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake" \
  -DANDROID_ABI=arm64-v8a -DANDROID_PLATFORM=android-26 \
  -DANDROID_STL=c++_shared \
  -DENABLE_LTO=OFF -DSURGE_SKIP_JUCE_FOR_RACK=TRUE \
  -DSURGE_BUILD_XT=OFF -DSURGE_BUILD_FX=OFF \
  -DSURGE_BUILD_CLAP=OFF -DSURGE_SKIP_DISTRIBUTION=TRUE \
  -DSURGE_SKIP_ODDSOUND_MTS=TRUE \
  -DHONEYDEW_SURGE_DIR="$ROOT"
cmake --build "$BUILD" --target honeydew_surge_xt --parallel "${SURGE_BUILD_JOBS:-3}"
LIB="$(find "$BUILD" -type f -name libhoneydew_surge_xt.so -print -quit)"
[ -n "$LIB" ] && [ -s "$LIB" ] || { echo 'Surge XT shared library not produced' >&2; exit 1; }
mkdir -p "$JNI"
cp "$LIB" "$JNI/libhoneydew_surge_xt.so"
# Android does not ship the NDK libc++_shared runtime as a system library.
CXX_RUNTIME="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so"
test -s "$CXX_RUNTIME"
cp "$CXX_RUNTIME" "$JNI/libc++_shared.so"
"$TOOLBIN/llvm-readelf" -h "$LIB" | grep -F 'AArch64'
"$TOOLBIN/llvm-readelf" --dyn-syms --wide "$LIB" | grep -F 'honeydew_surge_create'
echo "Original Surge XT native DSP ready: $JNI/libhoneydew_surge_xt.so"
