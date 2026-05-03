# Autofish — Android Mobile Control via Control

set dotenv-load := true

ANDROID_HOME := env("ANDROID_HOME", "~/Android/Sdk")
GRADLE := "./gradlew"
GRADLE_FLAGS := env("GRADLE_FLAGS", "")
ADB_DEVICE := env("ADB_DEVICE", "")
ADB := if ADB_DEVICE != "" { "adb -s " + ADB_DEVICE } else { "adb" }
SHIZUKU_VERSION := "13.6.0"
SHIZUKU_APK := "shizuku-v" + SHIZUKU_VERSION + ".apk"
SHIZUKU_URL := "https://github.com/RikkaApps/Shizuku/releases/download/v13.6.0/shizuku-v13.6.0.r1086.2650830c-release.apk"

APP_ID := "com.memohai.autofish"
APP_ID_DEBUG := APP_ID + ".debug"
EMULATOR_NAME := "autofish_test"
EMULATOR_DEVICE := "pixel_6"
EMULATOR_API := "34"
EMULATOR_IMAGE := "system-images;android-" + EMULATOR_API + ";google_apis;x86_64"
EMULATOR_RAM := "1024"
DEFAULT_PORT := "8080"

# ─── Help ────────────────────────────────────────────────────────────────────

# List all available recipes
default:
    @just --list

# ─── Build ───────────────────────────────────────────────────────────────────

# Build debug APK
build:
    {{ GRADLE }} {{ GRADLE_FLAGS }} assembleDebug

# Build debug APK without keeping Gradle daemon alive
build-once:
    {{ GRADLE }} --no-daemon assembleDebug

# Build release APK
build-release:
    {{ GRADLE }} {{ GRADLE_FLAGS }} assembleRelease

# Clean build artifacts
clean:
    {{ GRADLE }} {{ GRADLE_FLAGS }} clean

# Stop Gradle daemons
gradle-stop:
    {{ GRADLE }} --stop

# ─── Testing ─────────────────────────────────────────────────────────────────

# Run unit tests
test:
    {{ GRADLE }} {{ GRADLE_FLAGS }} :app:test

# Run integration tests only
test-integration:
    {{ GRADLE }} {{ GRADLE_FLAGS }} :app:testDebugUnitTest --tests "com.memohai.autofish.integration.*"

# Run cargo check for the CLI crate
cli-check:
    cargo check -q --manifest-path cli/Cargo.toml

# Run CLI tests
cli-test:
    cargo test -q --manifest-path cli/Cargo.toml

# Check CLI formatting
cli-fmt:
    cargo fmt --check --manifest-path cli/Cargo.toml

# Format CLI sources
cli-fmt-fix:
    cargo fmt --manifest-path cli/Cargo.toml

# Run clippy for the CLI crate
cli-clippy:
    cargo clippy --manifest-path cli/Cargo.toml --all-targets --all-features -- -D warnings

# Run CLI lint checks
cli-lint: cli-fmt cli-clippy

# Run all CLI quality checks
cli-quality: cli-check cli-test cli-lint

# ─── Linting ─────────────────────────────────────────────────────────────────

# Run all linters (ktlint + detekt)
lint:
    {{ GRADLE }} {{ GRADLE_FLAGS }} ktlintCheck detekt

# Auto-fix linting issues
lint-fix:
    {{ GRADLE }} {{ GRADLE_FLAGS }} ktlintFormat

# ─── Device Management ──────────────────────────────────────────────────────

# Install debug APK on connected device/emulator
install: build
    {{ GRADLE }} {{ GRADLE_FLAGS }} installDebug

# Install release APK
install-release: build-release
    {{ GRADLE }} {{ GRADLE_FLAGS }} installRelease

# Uninstall app from device
uninstall:
    -{{ ADB }} uninstall {{ APP_ID }}
    -{{ ADB }} uninstall {{ APP_ID_DEBUG }}

# Grant permissions via adb (accessibility + notifications)
grant-permissions:
    @echo "=== Granting permissions via adb ==="
    @echo ""
    @echo "1. Enabling Accessibility Service..."
    {{ ADB }} shell settings put secure enabled_accessibility_services \
        {{ APP_ID_DEBUG }}/com.memohai.autofish.services.accessibility.AutoFishAccessibilityService
    @echo "   Done."
    @echo ""
    @echo "2. Granting POST_NOTIFICATIONS permission..."
    {{ ADB }} shell pm grant {{ APP_ID_DEBUG }} android.permission.POST_NOTIFICATIONS
    @echo "   Done."

# Launch MainActivity on device
start:
    {{ ADB }} shell am start -n {{ APP_ID_DEBUG }}/{{ APP_ID }}.ui.MainActivity

# Set up adb port forwarding (device:8080 -> host:8080)
forward-port port=DEFAULT_PORT:
    {{ ADB }} forward tcp:{{ port }} tcp:{{ port }}
    @echo "Port forwarding: localhost:{{ port }} -> device:{{ port }}"

# ─── Emulator Management ────────────────────────────────────────────────────

# Create AVD for testing
setup-emulator:
    @echo "Creating AVD '{{ EMULATOR_NAME }}'..."
    @echo "Ensure system image is installed: sdkmanager '{{ EMULATOR_IMAGE }}'"
    avdmanager create avd \
        -n {{ EMULATOR_NAME }} \
        -k "{{ EMULATOR_IMAGE }}" \
        --device "{{ EMULATOR_DEVICE }}" \
        --force
    @echo "AVD '{{ EMULATOR_NAME }}' created."

# Start emulator headless (no window)
emulator-headless ram=EMULATOR_RAM:
    @echo "Starting emulator '{{ EMULATOR_NAME }}' (headless, RAM={{ ram }}MB)..."
    emulator -avd {{ EMULATOR_NAME }} -memory {{ ram }} -no-snapshot -no-window -no-audio -no-metrics &
    @echo "Waiting for emulator to boot..."
    {{ ADB }} wait-for-device
    @while [ "$({{ ADB }} shell getprop sys.boot_completed 2>/dev/null)" != "1" ]; do sleep 2; done
    @echo "Emulator is ready."

# Start emulator with graphical UI
emulator-ui ram=EMULATOR_RAM:
    @echo "Starting emulator '{{ EMULATOR_NAME }}' with UI (RAM={{ ram }}MB)..."
    QT_QPA_PLATFORM=xcb emulator -avd {{ EMULATOR_NAME }} -memory {{ ram }} -no-snapshot -no-metrics &
    @echo "Waiting for emulator to boot..."
    {{ ADB }} wait-for-device
    @while [ "$({{ ADB }} shell getprop sys.boot_completed 2>/dev/null)" != "1" ]; do sleep 2; done
    @echo "Emulator is ready."

# Stop running emulator
emulator-stop:
    -{{ ADB }} -s emulator-5554 emu kill
    @echo "Emulator stopped."

# ─── Shizuku ─────────────────────────────────────────────────────────────────

# Download Shizuku APK
shizuku-download:
    @if [ ! -f "{{ SHIZUKU_APK }}" ]; then \
        echo "Downloading Shizuku v{{ SHIZUKU_VERSION }}..."; \
        wget -cO "{{ SHIZUKU_APK }}" "{{ SHIZUKU_URL }}"; \
    else \
        echo "{{ SHIZUKU_APK }} already exists."; \
    fi

# Install Shizuku on device
shizuku-install: shizuku-download
    {{ ADB }} install -r "{{ SHIZUKU_APK }}"
    @echo "Shizuku installed."

# Start Shizuku service via adb
shizuku-start:
    {{ ADB }} shell sh /sdcard/Android/data/moe.shizuku.privileged.api/start.sh
    @echo "Shizuku service started."

# Check Shizuku status
shizuku-status:
    @echo "=== Shizuku Status ==="
    @{{ ADB }} shell dumpsys activity services | grep -i shizuku || echo "Shizuku service not found"
    @echo ""
    @{{ ADB }} shell pm list packages | grep shizuku || echo "Shizuku not installed"

# Full Shizuku setup: download + install + start
shizuku-setup: shizuku-install shizuku-start

# ─── Device Selection ────────────────────────────────────────────────────────

# List connected devices
devices:
    @adb devices -l

# ─── Logging ─────────────────────────────────────────────────────────────────

# Show app logs (filtered by autofish tag)
logs:
    {{ ADB }} logcat -s "autofish:*"

# Clear logcat buffer
logs-clear:
    {{ ADB }} logcat -c
    @echo "Logcat buffer cleared."

# ─── Environment ─────────────────────────────────────────────────────────────

# Check required development tools
check:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Checking required tools..."
    echo ""
    echo "  ANDROID_HOME = {{ ANDROID_HOME }}"
    if [ ! -d "{{ ANDROID_HOME }}" ]; then
        echo "  [WARN] ANDROID_HOME directory does not exist"
    fi
    if command -v java >/dev/null 2>&1; then
        echo "  [OK] Java $(java -version 2>&1 | head -1 | awk -F'"' '{print $2}')"
    else
        echo "  [MISSING] Java (JDK 17 required)"
    fi
    if [ -f "{{ GRADLE }}" ]; then
        echo "  [OK] Gradle wrapper found"
    else
        echo "  [MISSING] Gradle wrapper (gradlew)"
    fi
    if command -v {{ ADB }} >/dev/null 2>&1; then
        echo "  [OK] $({{ ADB }} version | head -1)"
    else
        echo "  [MISSING] adb (Android Debug Bridge)"
    fi
    echo ""
    echo "Done."

# ─── All-in-One ──────────────────────────────────────────────────────────────

# Full workflow: clean, build, lint, test
all: clean build lint test

# Quick deploy: build + install + start
deploy: install start
