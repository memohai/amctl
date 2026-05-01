# CLI Memory

`af` local memory stored in SQLite (`AF_DB`). Three layers: automatic execution tracking, agent-driven notes, structured event evidence.

## Data Model

- **`session_state`**: per-session observation cache (app, activity, page fingerprint, mode, has_webview, node_reliability, ref_version). Updated by observe commands. This is last-observed cache, not ground truth.
- **`events`**: append-only log of `act`/`verify`/`recover` with page fingerprint, failure cause, evidence, duration.
- **`transitions`**: auto-tracked act → verify pairs with pre/post page context, success/verified/failure counters. Three-tier query: page → activity → app.
- **`recoveries`**: auto-tracked recovery strategies linked to failure causes, with success/failure counters.
- **`notes`**: append-only agent-driven knowledge. Multiple notes with same `(app, topic)` coexist as historical chain.
- **`artifacts`**: saved large outputs such as screenshots and full observe payloads. SQLite stores metadata and file path; payload bytes live on disk.

`events`, `transitions`, `recoveries`, and `notes` persist across sessions. `session_state` is session-scoped only.
`artifacts` also persist across sessions when `AF_DB` is reused.

## Page Fingerprint

Stable, human-readable page identifier:

```
act=com.android.settings/.SubSettings|mode=SYSTEM_API|wv=0|rid=settings_main_pane,recycler_view|cls=RecyclerView,Toolbar
```

Components: `activity`, `mode`, `has_webview`, res_id anchors (up to 8), class_name anchors (up to 6, fallback).
Excluded: `text`, `desc`, `row_count`, dynamic list content.

## Commands

`af memory ...` is local-only and only needs memory configured via `memory.db`, `AF_DB`, or `--memory-db`.
Use global `--session` for `memory context` / `memory save`; use `--for-session` to filter `memory log` / `memory stats`.

```bash
af --session wf-1 memory save --app <pkg> --topic "nav/home-to-wifi" --content "..."
af memory search --app <pkg>
af memory search --topic "nav/" --query "wifi"
af memory delete --id 42
af memory experience --app <pkg> --activity <act> --page-fp "<fingerprint>"
af memory experience --app <pkg> --failure-cause REF_ALIAS_STALE
af --session wf-1 memory context
af memory log --for-session wf-1 --limit 20
af memory stats --for-session wf-1
```

## Artifact Storage

Large observe results are saved as artifacts instead of being inlined into CLI output.

- `observe screenshot` writes an image file and records artifact metadata.
- `observe screen --full` writes the full JSON payload as an artifact.
- `observe page` writes large screen payloads as artifacts and keeps only light metadata inline.

Artifact file location precedence:

1. Explicit CLI path such as `--save-file` or `--save-dir`
2. Per-kind env vars: `AF_SCREEN_FILE`, `AF_SCREENSHOT_FILE`, `AF_PAGE_DIR`
3. Config file keys: `artifacts.screen_file`, `artifacts.screenshot_file`, `artifacts.page_dir`
4. Shared root: `AF_ARTIFACT_DIR` or `artifacts.dir`

The SQLite `artifacts` table stores `session`, `trace_id`, `kind`, `mime_type`, `file_path`, `size_bytes`, and `content_hash`.

## Observation Cache

Observe commands update session cache with quality-based overwrite. Failed observations are ignored.

| Command | Updates | Generates Fingerprint |
|---|---|---|
| `observe top` | app + activity only | No |
| `observe screen` | Full cache, source=screen | Yes (always overwrites) |
| `observe refs` | Full cache, source=refs | Yes (only if activity changed, source ≠ screen, or fingerprint empty) |
| `observe page` | Full cache when identity reliable | Yes (screen > refs) |

`observe page` always returns base context (`topActivity`, `mode`, `hasWebView`, `nodeReliability`). `topActivity` is `null` when the service cannot confirm stability; in that case, the cache is not updated.

## Automatic Recording

**Events**: `act`, `verify`, `recover` → one row with session, app, activity, page fingerprint, status, failure cause, evidence, duration.

**Transitions**: When `verify` follows an `act` in the same session, a transition is closed: pre-page (from act's context) → action → post-page (from verify's context). Only closed when the verify actually has fresh post-context.

**Recoveries**: When `recover` follows a failed event in the same session, linked to the failure cause.

**Not recorded**: full command result JSON, `observe screenshot/overlay/page`, `health`, `memory`.

## Known Limits

- WebView-heavy pages: `nodeReliability=low`; prefer `verify text-contains` over node/ref assumptions.
- After `act`/`recover`, cached fingerprint is stale until the next observe.
- Dynamic UI can invalidate refs quickly; handle stale/unobserved ref errors by re-observing.
