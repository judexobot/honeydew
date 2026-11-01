#!/bin/sh
# Build an arm64 APK with the bundled sounds. Release signing uses environment variables, never checked-in keys.
set -eu
cd "$(dirname "$0")/.."

variant=debug
task=assembleDebug
case ${1:-} in
    '') ;;
    --release) variant=release; task=assembleRelease ;;
    *) echo "usage: tools/android-dist.sh [--release]" >&2; exit 2 ;;
esac
if [ "$variant" = release ]; then
    : "${ANDROID_KEYSTORE_FILE:?set the path to your release keystore}"
    : "${ANDROID_KEYSTORE_PASSWORD:?set your keystore password}"
    : "${ANDROID_KEY_ALIAS:?set your signing key alias}"
    : "${ANDROID_KEY_PASSWORD:?set your signing key password}"
    [ -f "$ANDROID_KEYSTORE_FILE" ] || { echo 'release keystore does not exist' >&2; exit 1; }
    ANDROID_KEYSTORE_FILE=$(cd "$(dirname "$ANDROID_KEYSTORE_FILE")" && pwd)/$(basename "$ANDROID_KEYSTORE_FILE")
    export ANDROID_KEYSTORE_FILE
fi
: "${ANDROID_HOME:?set ANDROID_HOME to the Android SDK directory}"
export ANDROID_SDK_ROOT=$ANDROID_HOME
export ANDROID_NDK_HOME=${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/28.0.13004108}
sdk_tools=$ANDROID_HOME/build-tools/36.0.0
for tool in cargo python3 unzip java keytool; do
    command -v "$tool" >/dev/null || { echo "needs $tool on PATH" >&2; exit 1; }
done

mkdir -p "${CARGO_TARGET_DIR:-target}"
export CARGO_TARGET_DIR=$(cd "${CARGO_TARGET_DIR:-target}" && pwd)
export HONEYDEW_ANDROID_BUILD_DIR=$CARGO_TARGET_DIR/android
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-4}
mkdir -p "$HONEYDEW_ANDROID_BUILD_DIR/assets" "$HONEYDEW_ANDROID_BUILD_DIR/jniLibs"
cat sounds/honeydew-sounds.zip.[0-9][0-9][0-9] > "$HONEYDEW_ANDROID_BUILD_DIR/sounds.zip"
# The .gz siblings are for web servers; Android reads the raw packs directly from its APK.
rm -rf "$HONEYDEW_ANDROID_BUILD_DIR/assets/content"
unzip -q "$HONEYDEW_ANDROID_BUILD_DIR/sounds.zip" 'content/*' -x '*.gz' -d "$HONEYDEW_ANDROID_BUILD_DIR/assets"
rm "$HONEYDEW_ANDROID_BUILD_DIR/sounds.zip"
# Ship the licenses and immutable corresponding-source location with the debug APK.
mkdir -p "$HONEYDEW_ANDROID_BUILD_DIR/assets/licenses"
cp LICENSE "$HONEYDEW_ANDROID_BUILD_DIR/assets/licenses/Honeydew-LICENSE.txt"
cp THIRD_PARTY_NOTICES.md "$HONEYDEW_ANDROID_BUILD_DIR/assets/licenses/THIRD_PARTY_NOTICES.md"
cp vendor/nuked-opn2/LICENSE "$HONEYDEW_ANDROID_BUILD_DIR/assets/licenses/Nuked-OPN2-LICENSE.txt"
printf 'Corresponding source: https://github.com/judexobot/honeydew/tree/%s\n' "$(git rev-parse HEAD)" > "$HONEYDEW_ANDROID_BUILD_DIR/assets/licenses/SOURCE.txt"
# signalsmith-stretch's C++ links libc++_shared.so, which the APK has to ship beside libhoneydew.so.
if [ "${HONEYDEW_SURGE_XT:-0}" = "1" ]; then
    [ -s "$HONEYDEW_ANDROID_BUILD_DIR/jniLibs/arm64-v8a/libhoneydew_surge_xt.so" ] || {
        echo 'Missing original Surge XT C++ library; run tools/android-surge-xt.sh first' >&2
        exit 1
    }
    cargo ndk -t arm64-v8a --platform 26 --link-libcxx-shared -o "$HONEYDEW_ANDROID_BUILD_DIR/jniLibs" rustc --release --locked --features surge-xt --lib -- -C link-arg=-Wl,-z,max-page-size=16384
else
    cargo ndk -t arm64-v8a --platform 26 --link-libcxx-shared -o "$HONEYDEW_ANDROID_BUILD_DIR/jniLibs" rustc --release --locked --lib -- -C link-arg=-Wl,-z,max-page-size=16384
fi
debug_keystore=$PWD/android/debug.keystore
if [ "$variant" = debug ] && [ ! -f "$debug_keystore" ]; then
    # Keep the existing signing identity when moving it out of disposable build output.
    if [ -f "$HONEYDEW_ANDROID_BUILD_DIR/debug.keystore" ]; then
        cp -p "$HONEYDEW_ANDROID_BUILD_DIR/debug.keystore" "$debug_keystore"
    else
        keytool -genkeypair -noprompt -keystore "$debug_keystore" \
            -storepass android -keypass android -alias androiddebugkey -keyalg RSA -keysize 2048 \
            -validity 10000 -dname 'CN=Android Debug,O=Android,C=US' >/dev/null 2>&1
    fi
fi
android/gradlew -p android --no-daemon "$task"

apk=$HONEYDEW_ANDROID_BUILD_DIR/outputs/apk/$variant/app-$variant.apk
"$sdk_tools/apksigner" verify "$apk"
"$sdk_tools/zipalign" -c -P 16 4 "$apk"
python3 - "$apk" <<'PY'
import base64
import hashlib
import json
import sys
import zipfile

# Android APK names are UTF-8 even when AAPT2 omits the ZIP UTF-8 flag.
# Python otherwise decodes the original µcomputer.fxp name as CP437.
with zipfile.ZipFile(sys.argv[1], metadata_encoding='utf-8') as apk:
    native_libs = ['libhoneydew.so', 'libc++_shared.so']
    if __import__('os').environ.get('HONEYDEW_SURGE_XT') == '1':
        native_libs.append('libhoneydew_surge_xt.so')
        # Presets may reference original Surge XT factory wavetable files.
        # Fail rather than shipping a silently degraded native synth.
        factory_wave = 'assets/surge-xt/wavetables/Basic/Sine.wt'
        assert len(apk.read(factory_wave)) > 100, 'missing original Surge XT wavetable assets'
        manifest = apk.read('assets/surge-xt/factory-manifest.tsv').decode('utf-8')
        for row in manifest.splitlines():
            sha, relative = row.split('\t', 1)
            assert hashlib.sha256(apk.read('assets/surge-xt/' + relative)).hexdigest() == sha, relative
        bank = json.loads(apk.read('assets/surge-xt/factory-bank.json'))
        assert bank['revision'] == '5a4828a33ac4b2e2b45671b34e9e0e3f0c614765'
        assert len(bank['patches']) == 641
        paths = [p['path'] for p in bank['patches']]
        assert paths == open('src/content/presets/surge_xt_factory.tsv').read().splitlines()
        chunks = bytearray()
        for patch in bank['patches']:
            original = apk.read('assets/surge-xt/patches_factory/' + patch['path'])
            assert hashlib.sha256(original).hexdigest() == patch['fxp_sha256'], patch['path']
            assert original[:4] == b'CcnK' and original[8:12] == b'FPCh'
            size = int.from_bytes(original[56:60], 'big')
            assert len(original) == size + 60
            chunks.extend(original[60:])
        # The 641 exact chunks must also be present in the native ELF bank
        # actually shipped to the phone, in the browser's exact index order.
        native = apk.read('lib/arm64-v8a/libhoneydew_surge_xt.so')
        assert bytes(chunks) in native, 'APK Surge library does not contain the verified FXP bank'
        print('Verified all 641 official FXPs, native bank order and every resource SHA-256')
    for lib in native_libs:
        assert apk.getinfo('lib/arm64-v8a/' + lib).compress_type == zipfile.ZIP_STORED, lib
    assert apk.read('assets/content/CREDITS.txt'), 'missing sound credits'
    assert apk.read('assets/licenses/Nuked-OPN2-LICENSE.txt'), 'missing chip-core license'
    assert apk.read('assets/licenses/SOURCE.txt'), 'missing corresponding source location'
    index = json.loads(apk.read('assets/content/index.json'))
    parts = {part['file']: part for entry in index['entries'] for part in entry['parts']}
    for part in parts.values():
        name = 'assets/content/' + part['file']
        assert apk.getinfo(name).compress_type == zipfile.ZIP_STORED, name
        data = apk.read(name)
        assert len(data) == part['bytes'], part['file']
        assert base64.b64encode(hashlib.sha256(data).digest()).decode() == part['sha256'], part['file']
    assert not any(name.endswith('.gz') for name in apk.namelist()), 'redundant web assets'
    print(f"Verified APK signature, alignment and {len(parts)} bundled sound packs")
PY
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
suffix=
[ "$variant" = release ] || suffix=-debug
out=$CARGO_TARGET_DIR/honeydew-android-$version-arm64$suffix.apk
cp "$apk" "$out"
echo "wrote $out"
