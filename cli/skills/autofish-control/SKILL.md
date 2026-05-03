---
name: autofish-control
description: Control Android devices with Autofish via the af CLI. Use this skill when you need reliable observe-act-verify loops, ref-based tapping, overlay management, screenshots, and recovery steps.
---

# Autofish Control Skill

Use deterministic control with evidence at every step.

## When To Use
- You need to navigate Android UI through Autofish (`af`).
- You need stable replay using refs (`@nK`) instead of coordinates.
- You need explicit verification after each action.

## Setup

```bash
af config set remote.url "http://<IP>:<PORT>"
af config set remote.token "<token>"
af config set memory.db "$HOME/.config/af/af.db"
af config set output.default "text"
af config set artifacts.dir "$HOME/.config/af/artifacts"
```

Copy `IP`, `PORT`, and `token` from the Autofish Android app. For USB port forwarding, use the forwarded local address, for example `http://127.0.0.1:<PORT>`.

If the device is connected with adb and the App is new enough to write a connection hint, prefer:

```bash
af connect usb --device <ADB_SERIAL>
```

`af connect usb` writes the forwarded `remote.url` and `connection.*` USB metadata, verifies `af health`, and does not write `remote.token`. Keep the token copied from the app for `observe`, `act`, `verify`, and `recover`.

Environment variables also work: `AF_URL`, `AF_TOKEN`, `AF_DB`, `AF_OUTPUT`, `AF_ARTIFACT_DIR`.

- `af health` only requires URL.
- `observe`, `act`, `verify`, `recover` require URL + token.
- `af memory ...` is local-only and requires memory enabled.
- `config list` and `config get remote.token` redact tokens as `<redacted>`.
- Use `--output json` when parsing results in scripts.
- Check `af --help` first if the installed CLI may be older than this skill.
- If config/env is unavailable, put remote flags after the command group: `af --session <session> observe --url <url> --token <token> page`.

Choose one session per task:

```bash
SESSION="task-name"
af --session "$SESSION" observe page --field screen --field refs --max-rows 80
```

Do not reuse the default `default` session across unrelated tasks.

## Workflow

Memory session rules:
- `memory context` and `memory save` use the global `--session`.
- `memory log` and `memory stats` filter with `--for-session <session>`.
- `memory search` and `memory experience` query cross-session history; do not treat global `--session` as a filter for them.

1. Baseline:
- `af --session <session> observe page --field screen --field refs --max-rows 80`
- `af --session <session> memory context`
- `af memory search --app <current-app>`
- `af memory experience --app <current-app> --activity <current-activity> --page-fp "<page-fingerprint>"`

2. One-step execution:
- Run exactly one action command.
- Run `af --session <session> observe page` (add `--field refs` if needed).
- Verify expected state.

3. Continue only from fresh observations. Do not run blind action chains.

4. After solving a non-trivial problem:
- `af --session <session> memory save --app <pkg> --topic "<category>/<name>" --content "<what you learned>"`

### Memory Loop

For experience learning, keep this exact order:

1. `observe` the starting page.
2. Run one `act`.
3. `observe` again.
4. Run one `verify`.

`act` and `recover` invalidate the cached page fingerprint. A transition is recorded only when a fresh successful observation happens after the action and before verification.

### observe page

Always returns: `topActivity`, `mode`, `hasWebView`, `nodeReliability`.

| `--field` | What it adds |
|---|---|
| `screen` (default) | Row counts, fingerprint rows, and a JSON artifact path for full rows |
| `refs` | Clickable ref aliases with `refVersion` |

Use the returned `screen.artifact` saved path when full page rows are needed. `observe screenshot` and overlay commands are diagnostics only; they do not refresh memory context.

`topActivity` is `null` when the service cannot confirm a stable value. Re-observe before acting.

## Tap Priority

1. `af --session <session> act tap --by ref --value @nK`
2. `af --session <session> act tap --by text|desc|resid --value "<value>" [--exact-match]`
3. `af --session <session> act tap --xy X,Y`

Other actions exist for non-tap flows: `swipe`, `text`, `launch`, `stop`, `key`, `back`, `home`.

## Refs

- Refresh refs on dynamic pages: `af --session <session> observe page --field refs`.
- If ref tap fails with stale/unobserved errors, re-observe once and retry once.
- Do not assume alias index stability after UI updates.
- Do not auto-pick among ambiguous matching candidates.

## Verify Priority

1. `af --session <session> verify top-activity --expected "<activity>" --mode contains`
2. `af --session <session> verify text-contains --text "<text>"`
3. `af --session <session> verify node-exists --by text|desc|resource_id|class --value "<value>" [--exact-match]`

`top-activity --mode` is `contains` or `equals`. `text-contains` is case-insensitive by default; use `--case-sensitive` only when exact case matters.

On WebView-heavy pages (`hasWebView=true` or `nodeReliability=low`), prefer `text-contains`. Do not rely on node/ref structure inside WebView content.

## Recovery

When state is uncertain:
1. `af --session <session> observe page --max-rows 120`
2. `af memory experience --app <pkg> --activity <act> --failure-cause <cause>`
3. Choose one recovery:
- `af --session <session> recover back --times 1` for modal/back-stack drift.
- `af --session <session> recover home` for lost navigation context.
- `af --session <session> recover relaunch --package <pkg>` for app reset.
4. `af memory log --for-session <session> --status failed --limit 5`
5. Re-run baseline.

## Diagnostics

```bash
af --session <session> observe overlay set --enable --mark-scope all --refresh on --refresh-interval-ms 800
af --session <session> observe screenshot --annotate --hide-overlay --max-marks 120 --mark-scope interactive
af --session <session> observe overlay set --disable --mark-scope all --refresh off
```

Artifacts still write to disk when memory is disabled, but DB artifact ids may be absent.

## Memory Topic Conventions

| Prefix | Purpose |
|---|---|
| `nav/` | Navigation paths between screens |
| `pitfall/` | Known problems and workarounds |
| `selector/` | Reliable selector strategies |
| `recovery/` | Recovery strategies that worked |

## Guidelines

- Favor refs over coordinates.
- Keep `--session` stable and unique per task.
- Each action must be followed by observation, then verification.
- Use `af memory experience` before acting on unfamiliar pages.
- Use `af --session <session> memory context` to inspect the current cached page fingerprint.
- Use `af memory log --for-session <session>` and `af memory stats --for-session <session>` to inspect failures and session quality.
