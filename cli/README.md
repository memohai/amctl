# af CLI (Rust)

`af` is the deterministic CLI executor for the Autofish Service.

## Install from npm

```bash
npm i -g @memohjs/af
```

## Build

```bash
cd cli
cargo build --release
```

## Run

```bash
SESSION="cli-demo"
af config list
af config set remote.url "http://<IP>:<PORT>"
af config set remote.token "<token>"
af config set memory.db "$HOME/.config/af/af.db"
af config set output.default "text"
af config set artifacts.dir "$HOME/.config/af/artifacts"
af config list
af app install --device <ADB_SERIAL>
af app uninstall --device <ADB_SERIAL> --dry-run
af connect usb --device <ADB_SERIAL>
af health
af --session "$SESSION" observe screenshot
af --session "$SESSION" observe screen --full
af --session "$SESSION" observe page --field screen --field refs --max-rows 80
af --session "$SESSION" act tap --by ref --value @n1
af --session "$SESSION" verify text-contains --text "Settings"
af memory search --app com.android.settings
af memory experience --app com.android.settings --activity com.android.settings/.Settings
```

## Command groups

- `health`
- `act`:
  - `tap` (preferred: `--by ref --value @nK`; coordinates: `--xy`; semantic: `--by --value [--exact-match]`)
  - `swipe`, `back`, `home`, `text`, `launch`, `stop`, `key`
- `observe`:
  - `page` (`--field screen|refs`; base context is always returned), `screen`, `refs`, `overlay`, `screenshot`, `top`
- `verify`:
  - `text-contains`, `top-activity`, `node-exists`
- `memory`:
  - `save`, `search`, `delete`, `experience`, `context`, `log`, `stats`
- `recover`:
  - `back`, `home`, `relaunch`
- `config`:
  - `list`, `get`, `set`, `unset`
- `app`:
  - `install` installs the official Autofish App over adb. Defaults to the App release matching the current CLI version.
  - `uninstall` removes the official Autofish App over adb.
- `connect`:
  - `usb` reads the App connection hint, creates adb forwarding, verifies `/health`, and writes local connection metadata.

See [docs/CLI_MEMORY.md](../docs/CLI_MEMORY.md) for memory design details.

## Output and exit code

Default output is stable multi-line `key=value` text. Use `--output json` when a script needs the old structured envelope.

- `status="ok"`: success, exit code `0`
- `status="failed"`: error, exit code `1`
- `status="interrupted"`: interrupted (SIGINT), exit code `130`

## Config

Config path resolution:

```bash
af --config ~/.config/af.toml config list # default path
AF_CONFIG=~/.config/af.toml af config get artifacts.dir
```

Supported env/config keys include:

- `AF_URL`, `AF_TOKEN`, `AF_DB`
- `AF_OUTPUT`
- `AF_ARTIFACT_DIR`
- `AF_SCREEN_FILE`
- `AF_SCREENSHOT_FILE`
- `AF_PAGE_DIR`

Equivalent config keys are `remote.url`, `remote.token`, `connection.transport`, `connection.usb.device`, `connection.usb.local_port`, `connection.usb.device_port`, `memory.db`, `output.default`, `artifacts.dir`, `artifacts.screen_file`, `artifacts.screenshot_file`, and `artifacts.page_dir`.

Example:

```bash
af config set remote.url "http://<IP>:<PORT>"
af config set remote.token "demo-token"
af config set memory.db "$HOME/.config/af/af.db"
af config set artifacts.dir "$HOME/.config/af/artifacts"
af config set output.default "text"
```

`config list` and `config get` show sensitive values such as `remote.token` as `<redacted>`.

## App Install

`af app install` is a local adb command. It does not require the Autofish HTTP service to be installed or running.

```bash
af app install --device <ADB_SERIAL>
af app install --device <ADB_SERIAL> --version 0.4.0 --dry-run
af app uninstall --device <ADB_SERIAL>
af app uninstall --device <ADB_SERIAL> --dry-run
```

By default, `--version current` installs the App release matching the current `af` CLI version. The command skips the install when the same version is already present, upgrades older versions, and refuses to downgrade unless `--force` is passed. To reinstall, run `af app uninstall` first. Downloaded APKs are cached under `$AF_CACHE_DIR` or `~/.cache/af`.

## USB Connect

`af connect usb` is a local adb command. It reads the Autofish App hint from the release or debug package under `/sdcard/Android/data/.../files/connection-hint.json`; the hint contains only package/version, service port, running status, and update time.

```bash
af connect usb --device <ADB_SERIAL>
af connect usb --device <ADB_SERIAL> --local-port 18081
af connect usb --device <ADB_SERIAL> --print-only
```

The command writes `remote.url`, `connection.transport=usb-forward`, `connection.usb.device`, `connection.usb.local_port`, and `connection.usb.device_port`. It does not write `remote.token`; `/health` remains token-free.

`af health` includes local connection metadata in output as `connection.transport`, `connection.device`, `connection.url`, `connection.localPort`, and `connection.devicePort`. When transport metadata is not configured, `connection.transport` is `unknown`.

## Artifact Output

Large observe payloads no longer inline image/base64 or full raw payloads in command output.

- `observe screenshot` saves an image artifact and returns summary fields plus saved path.
- `observe screen --full` saves the full JSON payload as an artifact.
- `observe page --field screen` saves the large screen slice as an artifact and keeps light metadata inline.
