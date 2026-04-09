# Quickstart

This guide is the fastest path to a working `af` setup.

## 1. Install the Android app

Install the latest APK from [GitHub Releases](https://github.com/memohai/Autofish/releases).

Open the app, then:

1. Enable accessibility permission for Autofish.
2. Enable Shizuku support.
3. Turn on **Service** on the home page.
4. Note the connection info shown by the app:
   - `IP`
   - `PORT`
   - `TOKEN`

## 2. Install the CLI

```bash
npm i -g @memohjs/af
```

## 3. Initialize CLI config

```bash
af config set remote.url "http://<IP>:<PORT>"
af config set remote.token "<TOKEN>"
af config set memory.db "$HOME/.config/af/af.db"
af config set output.default "text"
af config set artifacts.dir "$HOME/.config/af/artifacts"
```

## 4. Run the first checks

Service health:

```bash
af health
```

Page observation:

```bash
af observe page --field screen --field refs --max-rows 80
```

Action:

```bash
af act tap --by ref --value @n1
```

Optional follow-up commands:

```bash
af memory search --app com.android.settings
af memory context
af observe screenshot
```

## 5. Where to go next

- CLI details: [../cli/README.md](../cli/README.md)
- Architecture: [ARCHITECTURE.md](./ARCHITECTURE.md)
- Memory model: [CLI_MEMORY.md](./CLI_MEMORY.md)
