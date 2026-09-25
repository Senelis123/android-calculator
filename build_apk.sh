#!/usr/bin/env bash
# Builds a signed release APK. Needs: Rust targets aarch64-linux-android + armv7-linux-androideabi,
# Android NDK (ANDROID_NDK), build-tools (BUILD_TOOLS), ANDROID_HOME with platform 35, JDK 17 and Gradle 8.9+.
set -euo pipefail
cd "$(dirname "$0")"
: "${ANDROID_NDK:?set ANDROID_NDK}" "${BUILD_TOOLS:?set BUILD_TOOLS}"
BIN="$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64/bin"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$BIN/aarch64-linux-android23-clang"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$BIN/armv7a-linux-androideabi23-clang"
export CC_aarch64_linux_android="$BIN/aarch64-linux-android23-clang" CC_armv7_linux_androideabi="$BIN/armv7a-linux-androideabi23-clang"
export AR_aarch64_linux_android="$BIN/llvm-ar" AR_armv7_linux_androideabi="$BIN/llvm-ar"
# Version scheme A.B.C.D: A = app version, B = very big update, C = small update, D = fix.
VERSION=$(tr -d '[:space:]' < VERSION)
IFS=. read -r VA VB VC VD <<<"$VERSION"
VERSION_CODE=$((VA * 1000000 + VB * 10000 + VC * 100 + VD))
echo "Building version $VERSION (code $VERSION_CODE), update repo: ${UPDATE_REPO:-Senelis123/android-calculator}"
cargo build --release --target aarch64-linux-android --target armv7-linux-androideabi

rm -rf build; mkdir -p build/jni/arm64-v8a build/jni/armeabi-v7a
cp target/aarch64-linux-android/release/libcalculator.so build/jni/arm64-v8a/
cp target/armv7-linux-androideabi/release/libcalculator.so build/jni/armeabi-v7a/
# Java add-on (widget, quick tile, floating calculator, voice, Gemini Nano) + packaging + signing via Gradle
KS="${KEYSTORE:-debug.keystore}"
[ -f "$KS" ] || keytool -genkeypair -keystore "$KS" -storepass android -keypass android -alias androiddebugkey \
    -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=Android Debug,O=Android,C=US" >/dev/null 2>&1
export KEYSTORE="$(cd "$(dirname "$KS")" && pwd)/$(basename "$KS")"
${GRADLE:-gradle} --no-daemon -q -p android assembleRelease
cp android/build/outputs/apk/release/calculator-release.apk build/calculator-rust.apk
"$BUILD_TOOLS/apksigner" verify --verbose build/calculator-rust.apk
echo "APK: $(pwd)/build/calculator-rust.apk"
