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
af config list
af config set remote.url "http://<IP>:<PORT>"
af config set remote.token "<token>"
af config set memory.db "$HOME/.config/af/af.db"
af config set output.default "text"
af config set artifacts.dir "$HOME/.config/af/artifacts"
af config list
af health
af observe screenshot
af observe screen --full
af observe page --field screen --field refs --max-rows 80
af act tap --by ref --value @n1
af verify text-contains --text "Settings"
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

See [`docs/CLI_MEMORY.md`](../docs/CLI_MEMORY.md) for memory design details.

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

Example:

```bash
af config set remote.url "http://<IP>:<PORT>"
af config set remote.token "demo-token"
af config set memory.db "$HOME/.config/af/af.db"
af config set artifacts.dir "$HOME/.config/af/artifacts"
af config set output.default "text"
```

`config list` and `config get` show sensitive values such as `remote.token` as `<redacted>`.

## Artifact Output

Large observe payloads no longer inline image/base64 or full raw payloads in command output.

- `observe screenshot` saves an image artifact and returns summary fields plus saved path.
- `observe screen --full` saves the full JSON payload as an artifact.
- `observe page --field screen` saves the large screen slice as an artifact and keeps light metadata inline.
