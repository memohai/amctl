# af CLI (Rust)

`af` is the deterministic CLI executor for the Autofish Service.

## Build

```bash
cd cli
cargo build --release
```

## Install from npm

```bash
# rc channel
npm i -g @memohjs/af@rc

# stable channel
npm i -g @memohjs/af
```

Then run:

```bash
af --help
```

Note: Linux npm binaries are currently published for musl targets.

## Run

You can pass connection settings via flags or environment variables.

### Option A: environment variables

```bash
export AF_URL="http://127.0.0.1:8081"
export AF_TOKEN="<token>"
export AF_DB="./af.db"

./target/release/af health
./target/release/af observe screen --max-rows 80 --fields id,text,desc,resId,flags
./target/release/af observe refs --max-rows 80
./target/release/af act tap --x 540 --y 1200
./target/release/af act tap --by text --value "Settings"
./target/release/af act tap --by ref --value @n1
./target/release/af act swipe 100,1200,900,1200 --duration 300
```

### Option B: explicit flags

```bash
./target/release/af --url http://127.0.0.1:8081 --token "<token>" health
./target/release/af --url http://127.0.0.1:8081 --token "<token>" observe top
```

## Command groups

- `health`
- `act`:
  - `tap` (coordinates: `--x --y`; semantic: `--by --value [--exact-match]`)
  - `swipe`, `back`, `home`, `text`, `launch`, `stop`, `key`
- `observe`:
  - `screen`, `refs`, `overlay`, `screenshot`, `top`
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

- `--url <URL>` (or `AF_URL`)
- `--token <TOKEN>` (or `AF_TOKEN`)
- `--timeout-ms <MS>`
- `--proxy <system|direct|auto>`
- `--trace-db <PATH>` (or `AF_DB`)
- `--no-trace`
- `--session <NAME>`
