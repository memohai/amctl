# amctl

English | [中文](./README_ZH.md)

amctl is an Android device control service with a deterministic CLI client (`amc`).

## Build from source requirements

If you want to build from source (APK or CLI), prepare:

- JDK 17
- Android SDK (with `adb`)
- Rust toolchain (`cargo`)
- `just`

Recommended environment:

- `ANDROID_HOME` points to your Android SDK path

## Quickstart

### 1) Deploy service on Android device

#### Option A: install a prebuilt APK

Install the APK, open the app, then do the following:

1. Enable accessibility permission for amctl.
2. In Home page, turn on **Service**.
3. Note the service connection info shown in app:
   - Device IP
   - Port
   - Token

#### Option B: install from local source build

```bash
just build
just install
```

Then follow the same 3 steps above.

### 2) Install and use `amc` CLI

Build from source:

```bash
cd cli
cargo build --release
```

Set environment variables (replace with your actual values):

```bash
export AMC_URL="http://<DEVICE_IP>:<PORT>"
export AMC_TOKEN="<TOKEN>"
export AMC_DB="./amc.db"
```

Run first commands:

```bash
./target/release/amc health
./target/release/amc observe top
./target/release/amc observe screen --max-rows 80 --fields id,text,desc,resId,flags
./target/release/amc act tap --x 540 --y 1200
```

## Common CLI commands

```bash
amc observe screenshot --annotate --max-marks 120
amc act swipe 100,1200,900,1200 --duration 300
amc verify text-contains --text "Settings"
amc verify node-exists --by text --value "Settings"
amc recover back --times 2
```

Notes:

- `--url` is required unless `AMC_URL` is set.
- `--token` is required for protected commands unless `AMC_TOKEN` is set.
- Command output is JSON (single line per command).

## Troubleshooting

### Service not reachable

- Confirm device and host are on the same network.
- Confirm app shows service status as running.
- Confirm `AMC_URL` uses the device IP shown in app.

### Unauthorized / auth errors

- Confirm `AMC_TOKEN` matches the token shown in app.
- Regenerate token in app and update local env vars.

### Accessibility-related failures

- Re-check accessibility permission in Android settings.
- Re-open app and verify service is still running.

## For developers

```bash
just check
just build
cd cli && cargo test
```

More docs:

- [CLI details](./cli/README.md)
- [Design docs](./docs/)
