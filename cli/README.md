# amc CLI (Rust)

`amc` is the deterministic CLI executor for the amctl REST service.

## Build

```bash
cd cli
cargo build --release
```

## Run

You can pass connection settings via flags or environment variables.

### Option A: environment variables

```bash
export AMC_URL="http://127.0.0.1:8081"
export AMC_TOKEN="<token>"
export AMC_DB="./amc.db"

./target/release/amc health
./target/release/amc observe screen --max-rows 80 --fields id,text,desc,resId,flags
./target/release/amc act tap --x 540 --y 1200
```

### Option B: explicit flags

```bash
./target/release/amc --url http://127.0.0.1:8081 --token "<token>" health
./target/release/amc --url http://127.0.0.1:8081 --token "<token>" observe top
```

## Command groups

- `health`
- `act`:
  - `tap`, `swipe`, `back`, `home`, `text`, `launch`, `stop`, `key`
- `observe`:
  - `screen`, `overlay`, `screenshot`, `top`
- `verify`:
  - `text-contains`, `top-activity`, `node-exists`
- `recover`:
  - `back`, `home`, `relaunch`

## Output and exit code

Each command prints one JSON line.

- `status="ok"`: success, exit code `0`
- `status="failed"`: error, exit code `1`
- `status="interrupted"`: interrupted (SIGINT), exit code `130`

## Common options

- `--url <URL>` (or `AMC_URL`)
- `--token <TOKEN>` (or `AMC_TOKEN`)
- `--timeout-ms <MS>`
- `--proxy <system|direct|auto>`
- `--trace-db <PATH>` (or `AMC_DB`)
- `--no-trace`
- `--session <NAME>`
