# Android Calculator (Rust)

Rust rewrite of the original Java calculator (version 2.4.0.0). The app itself is Rust: it runs as an
Android `NativeActivity` (`android-activity` crate) and draws the UI itself. A small optional Java add-on
(`android/src/main/java`) adds what needs Android classes: home screen widget, quick settings tile,
floating calculator, voice input, text-to-speech/TalkBack announcements, background update check and
Gemini Nano (ML Kit GenAI Prompt API, on-device only). If an add-on feature is missing, the Rust app keeps working.

Features: basic calculator (same behavior as the Java app), operator precedence with live preview,
scientific mode (brackets, powers, roots, trig, logs, factorial, DEG/RAD), equations with x (linear and
quadratic), memory keys, history, VAT/tip calculator, unit, currency, cryptocurrency and stock converter (ECB rates plus market quotes),
solving a problem from a photo with on-device OCR (`ocrs`, no internet or API key), dark theme,
vibration, copy/paste, Lithuanian number format, landscape layout, auto-update from GitHub releases.
2.4: 7 languages (English default), more scientific functions, fractions, complex numbers, cubic
equations, inequalities, graphs, 16 tools (loan, salary, dates, statistics, matrices...), word problems
(Gemini Nano when the phone supports it, otherwise an offline reader), history search/notes/backup,
look settings (colors, contrast, sizes, one-hand mode, icon), beta update channel.

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
    # needs JDK 17, Gradle 8.9+, ANDROID_HOME with platforms;android-35
    ANDROID_NDK=/path/to/ndk BUILD_TOOLS=/path/to/build-tools/35.0.0 ./build_apk.sh

Output: `build/calculator-rust.apk` (signed with `KEYSTORE` and `KEY_ALIAS`; local default is a debug key, arm64-v8a + armeabi-v7a, Android 6.0+).

Tests: `cargo test`. Screens on a PC: `cargo run --release --example screens -- out.png 1080x2400 nav:settings`.

## Market converter

The Markets screen shows informational cryptocurrency and stock quotes on demand; it clears prior values if a refresh fails and does not show cached prices. Cryptocurrency quotes come from CoinGecko; stock quotes use Yahoo Finance (query1, then query2 fallback). Stock quotes may be rate-limited or unavailable; prices can be delayed.

## 2.4.0.0 additions

- Stocks and cryptocurrency quote screen (informational only), with refresh and 24h change.
- Angle and fuel-economy conversion.
- More expression/parser edge-case tests.
- Exact-fraction result toggle.
- CI checks for formatting, Clippy and tests.
- Releases from 2.4 onward use a dedicated release keystore in GitHub Actions and publish an APK SHA-256 checksum. Because 2.3.x was signed with a different key, Android cannot install 2.4 over 2.3.x: export any needed data, uninstall 2.3.x (which deletes local data), then install 2.4. Subsequent releases signed with the same new key can update normally.
- Market data may be delayed or rate-limited; the calculator does not provide trading recommendations.

### Release secrets

Configure these repository Actions secrets before publishing:
RELEASE_KEYSTORE_BASE64, RELEASE_KEYSTORE_PASSWORD, RELEASE_KEY_ALIAS.
Do not commit a production or release keystore to the repository.
