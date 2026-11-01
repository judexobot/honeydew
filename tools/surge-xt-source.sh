#!/usr/bin/env bash
# Source this from the native and Android build scripts. The checkout is a
# disposable build dependency, verified against upstream, never a personal fork.
set -euo pipefail
SURGE_REV=5a4828a33ac4b2e2b45671b34e9e0e3f0c614765
SURGE_SRC="${SURGE_SRC:-$TARGET/surge-xt-source}"
if [ ! -d "$SURGE_SRC/.git" ]; then
  mkdir -p "$SURGE_SRC"
  git -C "$SURGE_SRC" init
  git -C "$SURGE_SRC" remote add origin https://github.com/surge-synthesizer/surge.git
fi
test "$(git -C "$SURGE_SRC" remote get-url origin)" = https://github.com/surge-synthesizer/surge.git
if ! git -C "$SURGE_SRC" cat-file -e "$SURGE_REV^{commit}" 2>/dev/null; then
  git -C "$SURGE_SRC" fetch --depth 1 origin "$SURGE_REV"
fi
git -C "$SURGE_SRC" checkout --detach "$SURGE_REV"
test "$(git -C "$SURGE_SRC" rev-parse HEAD)" = "$SURGE_REV"
# Only the dependencies of surge-common. Desktop UI and plugin wrappers are
# excluded, while Lua, all original oscillators, filters and FX stay enabled.
git -C "$SURGE_SRC" submodule update --init --depth 1 --jobs "${SURGE_BUILD_JOBS:-2}" -- \
  libs/PEGTL libs/eurorack/eurorack libs/fmt libs/luajitlib/LuaJIT \
  libs/luajitlib/cmake libs/pffft libs/r8brain-free-src libs/simde \
  libs/sst/sst-basic-blocks libs/sst/sst-cmake libs/sst/sst-cpputils \
  libs/sst/sst-effects libs/sst/sst-filters libs/sst/sst-plugininfra \
  libs/sst/sst-waveshapers libs/tuning-library libs/zstd
git -C "$SURGE_SRC/libs/sst/sst-plugininfra" submodule update --init --recursive --depth 1 --jobs 2
python3 "$ROOT/tools/surge-xt-prepare.py" "$SURGE_SRC" "$ROOT"
