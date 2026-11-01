#!/usr/bin/env bash
# Build the original surge-common for Android ARM64, including Lua and all FX.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to the installed Android NDK}"
ROOT="$PWD"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
mkdir -p "$TARGET"
TARGET="$(cd "$TARGET" && pwd)"
source "$ROOT/tools/surge-xt-source.sh"
BUILD="$TARGET/surge-xt-android"
JNI="$TARGET/android/jniLibs/arm64-v8a"

python3 tools/surge-xt-bank.py "$SURGE_SRC" "$BUILD/generated" \
  src/content/presets/surge_xt_factory.tsv --assets "$TARGET/android/assets/surge-xt"

# Upstream's LuaJIT CMake helper invokes the HOST compiler. Supply its genuine
# AArch64 VM/archive, retaining Formula LFO and Lua wavetable functionality.
TOOLBIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
LUA_CC="$TOOLBIN/aarch64-linux-android26-clang"
LUA_AR="$TOOLBIN/llvm-ar"
LUA_SRC="$SURGE_SRC/libs/luajitlib/LuaJIT/src"
LUA_DST="$BUILD/libs/luajitlib/LuaJIT"
test -x "$LUA_CC"
make -C "$LUA_SRC" clean
make -C "$LUA_SRC" --jobs "${SURGE_BUILD_JOBS:-2}" amalg \
  BUILDMODE=static HOST_CC=cc TARGET_SYS=Linux \
  TARGET_CC="$LUA_CC" TARGET_STCC="$LUA_CC" TARGET_LD="$LUA_CC" \
  TARGET_AR="$LUA_AR rcus" TARGET_STRIP="$TOOLBIN/llvm-strip" \
  TARGET_CFLAGS="-fPIC -fvisibility=hidden"
mkdir -p "$LUA_DST/bin" "$LUA_DST/include"
cp "$LUA_SRC/libluajit.a" "$LUA_DST/bin/libluajit.a"
cp "$LUA_SRC/"*.h "$LUA_SRC/"*.hpp "$LUA_DST/include/"
"$LUA_AR" p "$LUA_DST/bin/libluajit.a" lj_vm.o > "$LUA_DST/lj_vm-check.o"
"$TOOLBIN/llvm-readelf" -h "$LUA_DST/lj_vm-check.o" | grep -F 'AArch64'
rm "$LUA_DST/lj_vm-check.o"

cmake -S "$SURGE_SRC" -B "$BUILD" -G Ninja \
  -DCMAKE_BUILD_TYPE=Release -DENABLE_LTO=OFF \
  -DCMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake" \
  -DANDROID_ABI=arm64-v8a -DANDROID_PLATFORM=android-26 -DANDROID_STL=c++_shared \
  -DSURGE_COMPILE_BLOCK_SIZE=32 -DSURGE_SKIP_JUCE_FOR_RACK=TRUE \
  -DSURGE_BUILD_XT=OFF -DSURGE_BUILD_FX=OFF -DSURGE_BUILD_CLAP=OFF \
  -DSURGE_SKIP_DISTRIBUTION=TRUE -DSURGE_SKIP_ODDSOUND_MTS=TRUE \
  -DHONEYDEW_SURGE_DIR="$ROOT"
cmake --build "$BUILD" --target honeydew_surge_xt --parallel "${SURGE_BUILD_JOBS:-2}"
LIB="$BUILD/libhoneydew_surge_xt.so"
test -s "$LIB"
mkdir -p "$JNI"
cp "$LIB" "$JNI/libhoneydew_surge_xt.so"
CXX_RUNTIME="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so"
cp "$CXX_RUNTIME" "$JNI/libc++_shared.so"
"$TOOLBIN/llvm-readelf" -h "$LIB" | grep -F 'AArch64'
exports="$("$TOOLBIN/llvm-readelf" --dyn-syms --wide "$LIB")"
while IFS= read -r symbol; do
  grep -Eq "[[:space:]]${symbol}([[:space:]]|$)" <<<"$exports" || {
    echo "Missing original Surge API export: $symbol" >&2; exit 1;
  }
done < native/surge-xt/exports.txt
"$TOOLBIN/llvm-readelf" -d "$LIB"
echo "Original Surge XT ARM64 ready; 641 complete patches and original resources verified"
