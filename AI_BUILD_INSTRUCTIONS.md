# AI BUILD INSTRUCTIONS — Android Calculator 2.4.0.0

You are the build agent. Your job is to turn this repository into the Android APK without changing product behavior.

## Source
- Repository: Senelis123/android-calculator
- Branch: release-2.4.0.0
- Version must be exactly 2.4.0.0
- APK output must be: build/calculator-rust.apk

## Environment
Install/use:
- JDK 17
- Gradle 8.9+
- Android SDK with platform android-35
- Android build-tools
- Android NDK
- Rust stable
- Rust Android targets:
  - aarch64-linux-android
  - armv7-linux-androideabi

## Verification before building
Run:
1. cargo fmt --all -- --check
2. cargo test --all
3. cargo clippy --all-targets --all-features -- -D warnings

Do not skip failures. First diagnose them; only make minimal build-fix changes that do not remove the new features.

## Build
Set:
- ANDROID_NDK to the installed NDK directory
- BUILD_TOOLS to the installed Android build-tools directory

For a local/debug-compatible build:
bash build_apk.sh

For a release-signed build:
- KEYSTORE must point to the release keystore
- KEYSTORE_PASSWORD must contain the keystore password
- KEY_ALIAS must contain the release alias

Then run:
KEYSTORE=/path/to/release.keystore KEYSTORE_PASSWORD="$RELEASE_KEYSTORE_PASSWORD" KEY_ALIAS="$RELEASE_KEY_ALIAS" ANDROID_NDK="$ANDROID_NDK" BUILD_TOOLS="$BUILD_TOOLS" bash build_apk.sh

The script should create:
build/calculator-rust.apk

## APK verification
Run:
"$BUILD_TOOLS/apksigner" verify --verbose build/calculator-rust.apk

Also verify that the final APK is non-empty and has versionName/versionCode matching 2.4.0.0.

## Important features that must remain
- Basic calculator
- Scientific calculator
- Fractions/exact fraction display
- Equations, inequalities and cubic solving
- Graph mode
- History, search, notes and export
- Unit/currency converter
- Angle and fuel-economy conversion
- Stocks and cryptocurrency informational quote screen
- OCR/photo calculator
- Voice/Gemini Nano integration
- Keyboard and touch gestures
- Dark/light/high-contrast UI
- Auto-update
- Android widget/tile/floating calculator

## Market screen
The market screen is informational only. It must not add trading or purchase actions.
Crypto data comes from the configured public market-data endpoint.
Stock data comes from the configured quote endpoint.
Network failures must leave the app usable and must not invent stale values.

## Release signing
Never commit a release keystore.
For GitHub Actions, the expected secrets are:
- RELEASE_KEYSTORE_BASE64
- RELEASE_KEYSTORE_PASSWORD
- RELEASE_KEY_ALIAS

The release workflow also publishes:
build/calculator-rust.apk.sha256

Do not print secret values into logs.

## Final result
Return:
- successful build confirmation
- exact APK path
- APK size
- signing verification result
- test/clippy results
- any remaining warnings or limitations
