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
- You need overlay/screenshot diagnostics for ambiguous UI states.

## Install

Install `af` CLI:

```bash
npm i -g @memohjs/af
```

Verify installation:

```bash
af --help
```

## Environment

```bash
export AF_URL="http://<host>:9998"
export AF_TOKEN="<token>"
export AF_DB="./af.db"
```

`AF_URL` is required.  
`AF_TOKEN` is required for protected calls.

## Workflow

1. Baseline:
- `af health`
- `af observe page --field screen --field refs --max-rows 80` — preferred single-call observe: base context + screen + refs in one call
- `af memory search --app <current-app>` — recall prior knowledge about this app
- `af memory experience --app <current-app> --activity <current-activity>` — recall past transitions/recoveries

2. One-step execution:
- Run exactly one action command.
- Re-observe state: `af observe page` (add `--field refs` if you need refs).
- Verify expected state.
- The transition is automatically recorded on verify.

3. Continue only from fresh observations.

4. After completing a task or learning something reusable:
- `af memory save --app <pkg> --topic "<category>/<name>" --content "<what you learned>"`

Do not run blind action chains.

If prior notes or experience exist for the current app, use them to inform your plan.

### observe page

Returns base context fields: `topActivity`, `mode`, `hasWebView`, `nodeReliability`.

| `--field` | What it adds |
|---|---|
| `screen` (default) | Full UI tree rows |
| `refs` | Clickable ref aliases with `refVersion` |

This is the most reliable single observe entrypoint for memory. `topActivity` can still be `null` if the service cannot confirm a stable value during the request; when that happens, do not assume page identity was refreshed.

## Preferred Tap Order

1. Ref tap: `af act tap --by ref --value @nK`
2. Semantic tap: `af act tap --by text|desc|resid --value "<value>" [--exact-match]`
3. Coordinate tap fallback: `af act tap --xy X,Y`

## Refs Handling

Refs are aliases from `af observe refs`.

- Refresh refs on dynamic pages.
- If ref tap fails with stale/unobserved errors, re-run `af observe page --field refs` (or `af observe refs`) and select again.
- Do not assume alias index stability after UI updates.

## Overlay

Get state:
```bash
af observe overlay get
```

Enable and configure:
```bash
af observe overlay set --enable --mark-scope all --refresh on --refresh-interval-ms 800
```

Disable:
```bash
af observe overlay set --disable --mark-scope all --refresh off
```

## Screenshot

Basic:
```bash
af observe screenshot
```

Annotated:
```bash
af observe screenshot --annotate --max-marks 120 --mark-scope interactive
```

## Swipe

```bash
af act swipe --from 100,1200 --to 900,1200 --duration 300
```

## Verification Priority

1. `af verify top-activity --expected "<activity>" --mode contains`
2. `af verify text-contains --text "<text>"`
3. `af verify node-exists --by text|desc|resid|class --value "<value>" [--exact-match]`

If node-based verification is unreliable on WebView-heavy pages (`nodeReliability=low`), prefer `text-contains` first.

## Recovery

When state is uncertain:
1. `af observe page --max-rows 120`
3. `af memory experience --app <pkg> --activity <act> --failure-cause <cause>` — check known recoveries
4. `af recover back --times 1` (or `2`)
5. `af memory log --session <current-session> --status failed --limit 5`
6. Re-run start checklist

## Examples

Open a screen item using refs:
```bash
af observe page --field refs --max-rows 80
af act tap --by ref --value @n3
af observe page
af verify text-contains --text "Settings"
```

Capture annotated evidence:
```bash
af observe overlay set --enable --mark-scope all --refresh on
af observe screenshot --annotate --max-marks 120 --mark-scope interactive
af observe overlay set --disable --mark-scope all --refresh off
```

## Memory Topic Conventions

| Prefix | Purpose |
|---|---|
| `nav/` | Navigation paths between screens |
| `pitfall/` | Known problems and workarounds |
| `selector/` | Reliable selector strategies for specific elements |
| `recovery/` | Recovery strategies that worked |

## Guidelines

- Favor refs over coordinates whenever possible.
- Keep `--session` stable across one task for event grouping.
- Keep each action followed by observation and verification (this also ensures transitions are recorded).
- Prefer deterministic checks (`top-activity`, `text-contains`) before deeper node checks.
- Treat recover commands as state-changing operations and re-baseline afterward.
- Save reusable knowledge with `af memory save` after solving non-trivial problems.
- Use `af memory experience` to check if the current page has known transitions or recoveries before acting.
- Use `af memory context` to inspect the current cached page fingerprint.
