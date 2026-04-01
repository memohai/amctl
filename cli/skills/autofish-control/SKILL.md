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
- `af observe top`
- `af observe screen --max-rows 80 --field id --field text --field desc --field resId --field flags`
- `af observe refs --max-rows 80`

2. One-step execution:
- Run exactly one action command.
- Re-observe state immediately.
- Verify expected state.

3. Continue only from fresh observations.

Do not run blind action chains.

## Preferred Tap Order

1. Ref tap: `af act tap --by ref --value @nK`
2. Semantic tap: `af act tap --by text|desc|resid --value "<value>" [--exact-match]`
3. Coordinate tap fallback: `af act tap --xy X,Y`

## Refs Handling

Refs are aliases from `af observe refs`.

- Refresh refs on dynamic pages.
- If ref tap fails with stale/unobserved errors, re-run `af observe refs` and select again.
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

If node-based verification is unreliable on WebView-heavy pages, prefer `text-contains` first.

## Recovery

When state is uncertain:
1. `af observe top`
2. `af observe screen --max-rows 120 --field id --field text --field desc --field resId --field flags`
3. `af recover back --times 1` (or `2`)
4. Re-run start checklist

## Examples

Open a screen item using refs:
```bash
af observe refs --max-rows 80
af act tap --by ref --value @n3
af verify text-contains --text "Settings"
```

Capture annotated evidence:
```bash
af observe overlay set --enable --mark-scope all --refresh on
af observe screenshot --annotate --max-marks 120 --mark-scope interactive
af observe overlay set --disable --mark-scope all --refresh off
```

## Guidelines

- Favor refs over coordinates whenever possible.
- Keep each action followed by observation and verification.
- Prefer deterministic checks (`top-activity`, `text-contains`) before deeper node checks.
- Treat recover commands as state-changing operations and re-baseline afterward.
