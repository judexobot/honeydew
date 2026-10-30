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
