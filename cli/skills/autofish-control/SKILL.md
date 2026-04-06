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

## Environment

```bash
export AF_URL="http://<host>:9998"
export AF_TOKEN="<token>"
export AF_DB="./af.db"
```

`af health` only requires `AF_URL`.
`observe`, `act`, `verify`, `recover` require `AF_URL` + `AF_TOKEN`.
`af memory ...` is local-only and only needs `AF_DB`.

## Workflow

1. Baseline:
- `af observe page --field screen --field refs --max-rows 80`
- `af memory search --app <current-app>`
- `af memory experience --app <current-app> --activity <current-activity>`

2. One-step execution:
- Run exactly one action command.
- `af observe page` (add `--field refs` if needed).
- Verify expected state.

3. Continue only from fresh observations. Do not run blind action chains.

4. After solving a non-trivial problem:
- `af memory save --app <pkg> --topic "<category>/<name>" --content "<what you learned>"`

### observe page

Always returns: `topActivity`, `mode`, `hasWebView`, `nodeReliability`.

| `--field` | What it adds |
|---|---|
| `screen` (default) | Full UI tree rows |
| `refs` | Clickable ref aliases with `refVersion` |

`topActivity` is `null` when the service cannot confirm a stable value; re-observe if that happens.

## Tap Priority

1. `af act tap --by ref --value @nK`
2. `af act tap --by text|desc|resid --value "<value>" [--exact-match]`
3. `af act tap --xy X,Y`

## Refs

- Refresh refs on dynamic pages: `af observe page --field refs`.
- If ref tap fails with stale/unobserved errors, re-observe and retry.
- Do not assume alias index stability after UI updates.

## Verify Priority

1. `af verify top-activity --expected "<activity>" --mode contains`
2. `af verify text-contains --text "<text>"`
3. `af verify node-exists --by text|desc|resid|class --value "<value>" [--exact-match]`

On WebView-heavy pages (`nodeReliability=low`), prefer `text-contains`.

## Recovery

When state is uncertain:
1. `af observe page --max-rows 120`
2. `af memory experience --app <pkg> --activity <act> --failure-cause <cause>`
3. `af recover back --times 1` (or `2`)
4. `af memory log --session <current-session> --status failed --limit 5`
5. Re-run baseline

## Diagnostics

```bash
af observe overlay set --enable --mark-scope all --refresh on --refresh-interval-ms 800
af observe screenshot --annotate --max-marks 120 --mark-scope interactive
af observe overlay set --disable --mark-scope all --refresh off
```

## Memory Topic Conventions

| Prefix | Purpose |
|---|---|
| `nav/` | Navigation paths between screens |
| `pitfall/` | Known problems and workarounds |
| `selector/` | Reliable selector strategies |
| `recovery/` | Recovery strategies that worked |

## Guidelines

- Favor refs over coordinates.
- Keep `--session` stable across one task.
- Each action must be followed by observation and verification.
- Use `af memory experience` before acting on unfamiliar pages.
- Use `af memory context` to inspect the current cached page fingerprint.
