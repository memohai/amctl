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
export AF_URL="http://127.0.0.1:8081"
export AF_TOKEN="<token>"
export AF_DB="./af.db"

af health
af observe page --field screen --field refs --max-rows 80
af memory context
af observe screen --max-rows 80 --field id --field text --field desc --field resId --field flags
af observe refs --max-rows 80
af memory search --app com.android.settings
af memory save --app com.android.settings --topic "nav/wifi" --content "Tap Network & Internet then Wi-Fi"
af memory experience --app com.android.settings --activity com.android.settings/.Settings
af memory context
af memory log --session demo --limit 10
af memory stats --session demo
af observe overlay get
af observe overlay set --enable --mark-scope all --refresh on
af observe screenshot --annotate --max-marks 120 --mark-scope interactive
af act tap --by ref --value @n1
af act tap --by text --value "Settings"
af act tap --xy 540,1200
af act swipe --from 100,1200 --to 900,1200 --duration 300
```

`health` only requires `AF_URL`. `observe`, `act`, `verify`, and `recover` require both `AF_URL` and `AF_TOKEN`. `af memory ...` is local-only and can run with just `AF_DB`, for example:

```bash
af memory log --session demo --limit 10
af memory context
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

## Local memory

`AF_DB` stores three layers of experience: automatic execution tracking, agent-driven notes, and structured event evidence.

- `af memory save / search / delete` — append-only knowledge notes
- `af memory experience` — query past transitions and recoveries (three-tier: page → activity → app)
- `af memory context` — view current session observation cache (page fingerprint)
- `af memory log / stats` — event log for `act` / `verify` / `recover` commands
- `memory` commands do not require `--url`; only `AF_DB` / `--memory-db` and `--session` matter locally
- `notes`, `events`, `transitions`, and `recoveries` persist across sessions; `session_state` is per-session observation cache only
- `observe page` is the preferred single-call observation entrypoint; `observe top / screen / refs` still update session cache with quality-based overwrite and no extra HTTP calls
- `topActivity` may be `null` on `observe page` when the service cannot confirm a stable value; in that case memory keeps the previous identity cache
- Transitions are auto-recorded when verify follows act; recoveries are auto-linked to prior failures
- `--session` groups events and transitions; notes are global across sessions
- WebView-heavy pages often report `nodeReliability=low`; prefer fresh observe and `verify text-contains` before relying on node/ref experience

## Output and exit code

Each command prints one JSON line.

- `status="ok"`: success, exit code `0`
- `status="failed"`: error, exit code `1`
- `status="interrupted"`: interrupted (SIGINT), exit code `130`
