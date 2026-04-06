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
af act tap --by ref --value @n1
af verify text-contains --text "Settings"
af memory search --app com.android.settings
af memory experience --app com.android.settings --activity com.android.settings/.Settings
```

`health` only requires `AF_URL`. `observe`, `act`, `verify`, and `recover` require both `AF_URL` and `AF_TOKEN`. `memory` is local-only and only needs `AF_DB`.

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

See [`docs/CLI_MEMORY.md`](../docs/CLI_MEMORY.md) for memory design details.

## Output and exit code

Each command prints one JSON line.

- `status="ok"`: success, exit code `0`
- `status="failed"`: error, exit code `1`
- `status="interrupted"`: interrupted (SIGINT), exit code `130`
