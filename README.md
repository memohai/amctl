# Autofish

English | [中文](./README_ZH.md)

Autofish is an Android device control service with a deterministic CLI client (`af`).

## Quickstart

### 1) Install app from GitHub Releases (APK)

Download and install the latest APK from [Releases](https://github.com/memohai/Autofish/releases)

Open the app, then:

1. Enable accessibility permission for Autofish.
2. Enable Shizuku support
3. In Home page, turn on **Service**.
4. Note the service connection info shown in app:
   - Device IP
   - Port
   - Token

### 2) Install `af` CLI from npm

```bash
npm i -g @memohjs/af
```

Initialize `af` config:

```bash
af config set remote.url "http://<IP>:<PORT>"
af config set remote.token "<TOKEN>"
af config set memory.db "$HOME/.config/af/af.db"
af config set output.default "text"
af config set artifacts.dir "$HOME/.config/af/artifacts"
```

Run first commands:

```bash
af health
af observe page --field screen --field refs --max-rows 80
af memory search --app com.android.settings
af memory context
af act tap --by ref --value @n1
af act tap --by text --value "Settings"
af act tap --xy 540,1200
```

## Build from source requirements (optional)

If you want to build APK or CLI from source, prepare:

- JDK 17
- Android SDK API 36
- Rust toolchain (`cargo`)
- `just`

Local source build example:

```bash
just build
just install
```

More docs:

- [CLI details](./cli/README.md)
- [Memory notes](./docs/CLI_MEMORY.md)
- [Design docs](./docs/)
