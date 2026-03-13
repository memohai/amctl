# amctl — Android Mobile Control via MCP

set dotenv-load := true

ANDROID_HOME := env("ANDROID_HOME", "~/Android/Sdk")
GRADLE := "./gradlew"
ADB := "adb"
APP_ID := "com.example.amctl"
APP_ID_DEBUG := APP_ID + ".debug"
EMULATOR_NAME := "amctl_test"
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
    {{ GRADLE }} assembleDebug

# Build release APK
build-release:
    {{ GRADLE }} assembleRelease

# Clean build artifacts
clean:
    {{ GRADLE }} clean

# ─── Testing ─────────────────────────────────────────────────────────────────

# Run unit tests
test:
    {{ GRADLE }} :app:test

# Run integration tests only
test-integration:
    {{ GRADLE }} :app:testDebugUnitTest --tests "com.example.amctl.integration.*"

# ─── Linting ─────────────────────────────────────────────────────────────────

# Run all linters (ktlint + detekt)
lint:
    {{ GRADLE }} ktlintCheck detekt

# Auto-fix linting issues
lint-fix:
    {{ GRADLE }} ktlintFormat

# ─── Device Management ──────────────────────────────────────────────────────

# Install debug APK on connected device/emulator
install: build
    {{ GRADLE }} installDebug

# Install release APK
install-release: build-release
    {{ GRADLE }} installRelease

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
        {{ APP_ID_DEBUG }}/com.example.amctl.services.accessibility.AmctlAccessibilityService
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
    emulator -avd {{ EMULATOR_NAME }} -memory {{ ram }} -no-snapshot -no-metrics &
    @echo "Waiting for emulator to boot..."
    {{ ADB }} wait-for-device
    @while [ "$({{ ADB }} shell getprop sys.boot_completed 2>/dev/null)" != "1" ]; do sleep 2; done
    @echo "Emulator is ready."

# Stop running emulator
emulator-stop:
    -{{ ADB }} -s emulator-5554 emu kill
    @echo "Emulator stopped."

# ─── Logging ─────────────────────────────────────────────────────────────────

# Show app logs (filtered by amctl tag)
logs:
    {{ ADB }} logcat -s "amctl:*"

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
