# AGENTS.md

## Project

amctl — Android device remote control via MCP protocol, targeting RhinoPi X1 (Qualcomm QCS8550 ARM64).

## Context

Read `docs/ARCHITECTURE.md` before making changes. It is the single source of truth for current architecture.

## Current Baseline

- Transport/API:
  - MCP service: `McpServerService` + `/mcp`
  - REST service: `RestServerService` + `/api/*`
- Execution path:
  - Prefer v2: Shizuku + system/shell path
  - Fallback to v1: Accessibility path
  - Routing entry: `ToolRouter`
- Architecture intent:
  - v2 is an internal implementation upgrade on top of v1 compatibility
  - Keep existing external tool interfaces stable unless design docs are updated first

## Rules

- Respond in Simplified Chinese
- Use `just` instead of `make` for build commands
- Do not add features not listed in design documents
- Hidden API reflection calls must have try-catch + fallback
- v2 changes must not alter tool interfaces; only swap internal implementation paths, keeping v1 as fallback

## Change Process

- If a requested feature is not in current design/plan docs:
  - Update relevant docs under `docs/` first (or in same change set), then implement.
  - If intentionally temporary, mark clearly as `experimental` in code/comments and PR description.
- For v2-related refactors:
  - Do not break existing MCP tool names/params/return schema.
  - Maintain runnable fallback path when v2 capability is unavailable.

## Quality Gates

- Before finishing a coding task, run at least:
  - `just build`
- When logic changes are introduced, add/update minimal tests in the same change when feasible.
- Never merge changes that introduce obvious startup blocking, reliability regressions, or loss of observability.
