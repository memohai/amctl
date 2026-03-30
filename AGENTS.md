# AGENTS.md

## Project

Autofish — Android device remote control via remote control protocol, targeting RhinoPi X1 (Qualcomm QCS8550 ARM64).

## Context

Read `docs/ARCHITECTURE.md` before making changes. It is the single source of truth for current architecture.

## Current Baseline

- Transport/API:
  - Service: `ServiceServerService` + `/api/*`
- Execution path:
  - Prefer: Shizuku + system/shell path
  - Fallback: Accessibility path
  - Routing entry: `ToolRouter`
- Architecture intent:
  - Keep existing external interfaces stable unless design docs are updated first

## Rules

- Respond in Simplified Chinese
- Use `just` instead of `make` for build commands
- Do not add features not listed in design documents
- Hidden API reflection calls must have try-catch + fallback
- Implementation changes must not alter external tool interfaces; keep fallback path runnable

## Change Process

- If a requested feature is not in current design/plan docs:
  - Update relevant docs under `docs/` first (or in same change set), then implement.
  - If intentionally temporary, mark clearly as `experimental` in code/comments and PR description.
- For internal refactors:
  - Do not break existing tool names/params/return schema.
  - Maintain runnable fallback path when preferred capability is unavailable.

## Quality Gates

- Before finishing a coding task, run at least:
  - `just build`
- When logic changes are introduced, add/update minimal tests in the same change when feasible.
- Never merge changes that introduce obvious startup blocking, reliability regressions, or loss of observability.
