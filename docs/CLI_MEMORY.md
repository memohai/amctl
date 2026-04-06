# CLI Memory

`af` local memory is a three-layer experience system: automatic execution tracking, agent-driven notes, and structured event evidence.

## Design Principles

- **Automatic experience tracking.** Transitions and recoveries are recorded automatically from act → verify/recover sequences. The agent benefits from accumulated experience without manual effort.
- **Tool stores facts, agent reasons.** No confidence scores, no built-in recommendations. The agent queries experience and decides what to do.
- **Zero execution overhead.** No extra HTTP calls for context capture; page fingerprint is computed from observe commands the agent already runs.
- **Session observation cache is not ground truth.** After an act/recover, the cached fingerprint is stale until the next observe. Transitions close only after a subsequent verify confirms the new state.

## Page Fingerprint

Each page is identified by a stable, human-readable fingerprint:

```
act=com.android.settings/.SubSettings|mode=SYSTEM_API|wv=0|rid=settings_main_pane,recycler_view|cls=RecyclerView,Toolbar
```

Components: `activity`, `mode`, `has_webview`, res_id anchors (up to 8), class_name anchors (up to 6, as fallback).

Excluded from fingerprint: `text`, `desc`, `row_count`, dynamic list content.

## Commands

### Knowledge notes (append-only)

```bash
# Save a note
af memory save --app com.android.settings --topic "nav/home-to-wifi" \
  --content "Tap 'Network & Internet' then tap 'Wi-Fi'"

# Search notes
af memory search --app com.android.settings
af memory search --topic "nav/"
af memory search --query "wifi"

# Delete a note by id
af memory delete --id 42
```

### Experience query (three-tier: page → activity → app)

```bash
# Query what transitions/recoveries are known from a page
af memory experience \
  --app com.android.settings \
  --activity com.android.settings/.Settings \
  --page-fp "act=com.android.settings/.Settings|mode=SYSTEM_API|wv=0|rid=main_pane"

# Query recoveries for a specific failure cause
af memory experience \
  --app com.android.settings \
  --failure-cause REF_ALIAS_STALE
```

### Session context

```bash
# View current observation cache
af memory context
```

### Event log

```bash
af memory log --session wf-1 --limit 20
af memory log --status failed --limit 10
af memory stats --session wf-1
```

## Observation Cache

Observe commands update the session observation cache with quality-based overwrite rules. Only successful observations update the cache; failed observe commands are ignored.

| Command | Updates | Generates Fingerprint |
|---|---|---|
| `observe top` | app + activity only | No |
| `observe screen` | Full cache + strongest fingerprint | Yes (overwrites always) |
| `observe refs` | Full cache + weaker fingerprint | Yes (if activity changed, current source is not `screen`, or fingerprint is empty) |
| `observe page` | Full cache when identity is reliable | Yes (screen > refs > top) |

### Atomic observation: `observe page`

`observe page` always returns the base context fields (`topActivity`, `mode`, `hasWebView`, `nodeReliability`) in the response shape. `topActivity` may be `null` when the service cannot confirm a stable value during the request. In that case, CLI memory does not refresh the cached page identity from `observe page`.

Use `--field screen` and/or `--field refs` to include heavy data slices. Default (no `--field`): `screen`.

Available fields: `screen`, `refs`.

Individual `observe top/screen/refs` commands still include `topActivity` for backward compatibility, but they should be treated as weaker observation inputs than `observe page`.

## Automatic Recording

### What is recorded

- **Events**: `act`, `verify`, `recover` → one row with session, app, activity, page fingerprint, status, failure cause, verify evidence, duration.
- **Transitions**: When a `verify` follows an `act` in the same session, the system closes a transition: pre-page (from act's cached context) → action → post-page (from verify's cached context). Stored with success/verified/failure counters.
- **Recoveries**: When a `recover` follows a failed event in the same session, the system links the recovery to the failure cause. Stored with success/failure counters.
- **Session cache**: `observe top/screen/refs/page` → update cached page context; no event row.

`events`, `transitions`, `recoveries`, and `notes` persist across sessions. `session_state` is the only session-scoped layer.

### What is NOT recorded

- Full command result JSON.
- `observe screenshot`, `observe overlay`, `health`, `memory` → nothing.
- `observe page` generates no event row; it only updates the session observation cache.
- Transitions are NOT closed at act time (stale cache problem); they close only when a subsequent verify provides post-context.

What is still stored from execution results:
- `failure_cause` for failed events
- structured `evidence_json` (especially verify evidence, and failure raw/error details when available)

## Topic Conventions

| Prefix | Purpose |
|---|---|
| `nav/` | Navigation paths between screens |
| `pitfall/` | Known problems and workarounds |
| `selector/` | Reliable selector strategies for specific elements |
| `recovery/` | Recovery strategies that worked |

## Notes

- `--session` groups events and transitions within one task; notes are global and persist across sessions.
- Notes are append-only: multiple notes with the same `(app, topic)` coexist as a historical chain.
- Event log is append-only with no automatic cleanup.

## Known Limits

- WebView-heavy pages often report `nodeReliability=low`; prefer `verify text-contains` and fresh observe over strong assumptions about node/ref stability.
- `session_state` is last-observed cache, not ground truth. After `act` / `recover`, the cached fingerprint is stale until the next reliable observe.
- Dynamic UI can invalidate refs quickly; stale/unobserved ref errors should be handled by re-observing before retrying.
