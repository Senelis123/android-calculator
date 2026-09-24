# Android Calculator (Rust)

Rust rewrite of the original Java calculator (version 2.2.0.0). 100% Rust, no Java/Kotlin: runs as an
Android `NativeActivity` (`android-activity` crate) and draws the UI itself.

Features: basic calculator (same behavior as the Java app), operator precedence with live preview,
scientific mode (brackets, powers, roots, trig, logs, factorial, DEG/RAD), equations with x (linear and
quadratic), memory keys, history, VAT/tip calculator, unit and currency converter (ECB rates),
solving a problem from a photo with on-device OCR (`ocrs`, no internet or API key), dark theme,
vibration, copy/paste, Lithuanian number format, landscape layout, auto-update from GitHub releases.

- `src/calc.rs` - basic calculator logic (1:1 port of the old `MainActivity.java`, with tests).
- `src/expr.rs` - expressions, equation solving, number formatting. `src/convert.rs` - units/currency.
- `src/ocr.rs` - photo text recognition (models in `models/`). `src/app.rs` - screens and state.
- `src/draw.rs` - drawing. `src/android.rs` - window, touch and Android services via JNI.
- `src/update.rs` - auto-updater (see below). Fonts: DejaVu Sans (bundled, free license).

## Auto-update

On every start the app asks GitHub for the latest release of `UPDATE_REPO`
(default `Senelis123/android-calculator`, set at build time). If its tag (e.g. `v2.2.0.1`) is newer than
the app version in `VERSION` and the release has an `.apk` file, a banner appears; tapping it
downloads the APK and opens Android's installer.

- The releases repo must be PUBLIC (the app has no GitHub token; private repos answer 404).
- Every release must be signed with the same key (`debug.keystore`), or Android refuses the update.
- Versions have four parts A.B.C.D: A = app version, B = very big update, C = small update, D = fix.
- To publish: change `VERSION` (e.g. 2.2.0.1), commit, then create tag `v2.2.0.1` (or a GitHub release with that tag).
  `.github/workflows/release.yml` builds and attaches the APK (needs the `KEYSTORE_BASE64` secret).

## Build

    rustup target add aarch64-linux-android armv7-linux-androideabi
    ANDROID_NDK=/path/to/ndk BUILD_TOOLS=/path/to/build-tools/34.0.0 \
    ANDROID_JAR=/path/to/platforms/android-35/android.jar ./build_apk.sh

Output: `build/calculator-rust.apk` (debug-signed, arm64-v8a + armeabi-v7a, Android 6.0+).

Tests: `cargo test`. Screenshot on a PC: `cargo run --example render_png -- out.png 1 + 2 =`.
