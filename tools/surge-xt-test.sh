#!/usr/bin/env bash
# Native audio evidence with the SAME bridge/core as the APK. No audio device.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
mkdir -p "$TARGET"
TARGET="$(cd "$TARGET" && pwd)"
source "$ROOT/tools/surge-xt-source.sh"
BUILD="$TARGET/surge-xt-host"
python3 tools/surge-xt-bank.py "$SURGE_SRC" "$BUILD/generated" \
  src/content/presets/surge_xt_factory.tsv --assets "$TARGET/android/assets/surge-xt"
cmake -S "$SURGE_SRC" -B "$BUILD" -G Ninja \
  -DCMAKE_BUILD_TYPE=Release -DENABLE_LTO=OFF \
  -DSURGE_COMPILE_BLOCK_SIZE=32 -DSURGE_SKIP_JUCE_FOR_RACK=TRUE \
  -DSURGE_BUILD_XT=OFF -DSURGE_BUILD_FX=OFF -DSURGE_BUILD_CLAP=OFF \
  -DSURGE_SKIP_DISTRIBUTION=TRUE -DSURGE_SKIP_ODDSOUND_MTS=TRUE \
  -DHONEYDEW_SURGE_APP_PATHS=ON -DHONEYDEW_SURGE_TESTS=ON \
  -DHONEYDEW_SURGE_DIR="$ROOT"
cmake --build "$BUILD" --target honeydew_surge_xt honeydew_surge_audio_test \
  --parallel "${SURGE_BUILD_JOBS:-2}"
export HONEYDEW_SURGE_LIBRARY="$BUILD/libhoneydew_surge_xt.so"
export HONEYDEW_SURGE_DATA_PATH="$TARGET/android/assets/surge-xt"
export HONEYDEW_SURGE_USER_PATH="$TARGET/surge-xt-private-user"
mkdir -p "$TARGET/surge-audio-qa"
cp "$BUILD/generated/factory-bank.json" "$TARGET/surge-audio-qa/"
# A child without a desktop login environment reproduces Android's path setup.
# Do not change the parent environment used by Rustup and the user's tools.
python3 - "$BUILD/honeydew_surge_audio_test" "$HONEYDEW_SURGE_DATA_PATH" \
  "$HONEYDEW_SURGE_USER_PATH" "$TARGET/surge-audio-qa" <<'PY'
import os, subprocess, sys
subprocess.run(sys.argv[1:], env={k: v for k, v in os.environ.items() if k != "HOME"}, check=True)
PY
cargo test --locked --features surge-xt --test surge_xt -- --test-threads=1
