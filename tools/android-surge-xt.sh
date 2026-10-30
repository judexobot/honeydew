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
git -C "$SRC" fetch --depth=1 origin "$REV"
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
echo "Original Surge XT native DSP ready: $JNI/libhoneydew_surge_xt.so"
