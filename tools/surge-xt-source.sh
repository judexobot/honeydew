#!/usr/bin/env bash
# Source this from the native and Android build scripts. The checkout is a
# disposable build dependency, verified against upstream, never a personal fork.
set -euo pipefail
SURGE_REV=5a4828a33ac4b2e2b45671b34e9e0e3f0c614765
SURGE_SRC="${SURGE_SRC:-$TARGET/surge-xt-source}"
SURGE_UPSTREAM=https://github.com/surge-synthesizer/surge.git
# rust-cache recursively removes non-Cargo files under target/. An empty .git
# directory then makes `git -C` discover Honeydew's parent repository instead.
# Validate the checkout root as well as its origin before any fetch or mutation.
if [ -e "$SURGE_SRC" ]; then
  source_root="$(git -C "$SURGE_SRC" rev-parse --show-toplevel 2>/dev/null || true)"
  source_origin="$(git -C "$SURGE_SRC" config --local --get remote.origin.url 2>/dev/null || true)"
  if [ "$source_root" != "$(cd "$SURGE_SRC" && pwd -P)" ] || [ "$source_origin" != "$SURGE_UPSTREAM" ]; then
    if [ "$SURGE_SRC" != "$TARGET/surge-xt-source" ]; then
      echo "SURGE_SRC must be its own checkout of the official Surge repository" >&2
      return 1
    fi
    echo "Discarding incomplete cached Surge dependency; restoring verified upstream"
    rm -rf -- "$SURGE_SRC"
  fi
fi
if [ ! -d "$SURGE_SRC/.git" ]; then
  mkdir -p "$SURGE_SRC"
  git -C "$SURGE_SRC" init
  git -C "$SURGE_SRC" remote add origin "$SURGE_UPSTREAM"
fi
echo "Verified official Surge source checkout"
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
