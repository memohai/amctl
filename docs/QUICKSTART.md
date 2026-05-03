# Quickstart

The shortest path from a fresh Android install to a working `af observe page`.

## 1. Install the CLI

```bash
npm i -g @memohjs/af
```

## 2. Install the Android app

Install the latest APK from [GitHub Releases](https://github.com/memohai/Autofish/releases).

If the device is connected with adb, the CLI can install the matching official App release:

```bash
af app install --device <ADB_SERIAL>
```

On the device:

1. Enable accessibility permission for Autofish.
2. Enable Shizuku support when available.
3. Turn on **Service** on the home page.
4. Copy the `af config` commands from the home page connection card.

If Shizuku is not running, Autofish can still use the accessibility fallback when the accessibility service is enabled.

## 3. Configure the connection

Run the `remote.url` and `remote.token` commands copied from the app, then add local defaults:

```bash
af config set memory.db "$HOME/.config/af/af.db"
af config set output.default "text"
af config set artifacts.dir "$HOME/.config/af/artifacts"
```

For USB, let the CLI read the App's non-sensitive connection hint and configure adb forwarding:

```bash
af connect usb --device <ADB_SERIAL>
```

`af connect usb` replaces the copied `remote.url` with a local forwarded URL and records USB metadata under `connection.*`. It does not write `remote.token`, so keep the token copied from the app for commands beyond `af health`.

## 4. Check the service

```bash
SESSION="quickstart"
af health
```

## 5. Observe the first page

```bash
af --session "$SESSION" observe page --field screen --field refs --max-rows 80
```

You should see page context plus clickable refs such as `@n1`, `@n2`, ...

## 6. Optional first action

Only act after a fresh observation, then observe again and verify:

```bash
af --session "$SESSION" act tap --by ref --value @n1
af --session "$SESSION" observe page --field screen --field refs --max-rows 80
af --session "$SESSION" verify text-contains --text "Settings"
```

## Next

- Full overview and skill setup: [../README.md](../README.md)
- CLI details: [../cli/README.md](../cli/README.md)
- Memory model: [CLI_MEMORY.md](./CLI_MEMORY.md)
- Architecture: [ARCHITECTURE.md](./ARCHITECTURE.md)
