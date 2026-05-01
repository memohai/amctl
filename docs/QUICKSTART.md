# Quickstart

The shortest path from a fresh Android install to a working `af observe page`.

## 1. Install the Android app

Install the latest APK from [GitHub Releases](https://github.com/memohai/Autofish/releases).

On the device:

1. Enable accessibility permission for Autofish.
2. Enable Shizuku support when available.
3. Turn on **Service** on the home page.
4. Copy the `IP`, `PORT`, and `TOKEN` shown by the app.

If Shizuku is not running, Autofish can still use the accessibility fallback when the accessibility service is enabled.

## 2. Install the CLI

```bash
npm i -g @memohjs/af
```

## 3. Configure the connection

```bash
af config set remote.url "http://<IP>:<PORT>"
af config set remote.token "<TOKEN>"
af config set memory.db "$HOME/.config/af/af.db"
af config set output.default "text"
af config set artifacts.dir "$HOME/.config/af/artifacts"
```

For USB port forwarding, forward the same port shown by the app and use `http://127.0.0.1:<PORT>` as `remote.url`.

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
