# Autofish

English | [中文](./README_ZH.md)

Autofish is an Android device control service with a deterministic CLI client (`af`).

## Quickstart

### 1) Install app from GitHub Releases (APK)

Download and install the latest APK from GitHub Releases:  
https://github.com/memohai/Autofish/releases

Open the app, then:

1. Enable accessibility permission for Autofish.
2. In Home page, turn on **Service**.
3. Note the service connection info shown in app:
   - Device IP
   - Port
   - Token

### 2) Install `af` CLI from npm

```bash
npm i -g @memohjs/af
```

Then verify:

```bash
af --help
```

Set environment variables:

```bash
export AF_URL="http://<DEVICE_IP>:<PORT>"
export AF_TOKEN="<TOKEN>"
export AF_DB="./af.db"
```

Run first commands:

```bash
af health
af observe top
af observe screen --max-rows 80 --fields id,text,desc,resId,flags
af observe refs --max-rows 80
af act tap --x 540 --y 1200
af act tap --by text --value "Settings"
```

## Build from source requirements (optional)

If you want to build APK or CLI from source, prepare:

- JDK 17
- Android SDK (with `adb`)
- Rust toolchain (`cargo`)
- `just`

Recommended environment:

- `ANDROID_HOME` points to your Android SDK path

Local source build example:

```bash
just build
just install
cd cli
cargo build --release
```

## Common CLI commands

```bash
af observe screenshot --annotate --max-marks 120
af act swipe 100,1200,900,1200 --duration 300
af act tap --by resid --value "com.android.settings:id/title" --exact-match
af observe refs --max-rows 120
af act tap --by ref --value @n3
af verify text-contains --text "Settings"
af verify node-exists --by text --value "Settings"
af recover back --times 2
```

Notes:

- If `AF_URL` or `AF_TOKEN` is not set, pass `--url` / `--token` explicitly.
- Command output is JSON (single line per command).

## For developers

```bash
just check
just build
cd cli && cargo test
```

More docs:

- [CLI details](./cli/README.md)
- [Design docs](./docs/)
