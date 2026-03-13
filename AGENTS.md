# AGENTS.md

## Project

amctl — Android device remote control via MCP protocol, targeting RhinoPi X1 (Qualcomm QCS8550 ARM64).

## Context

Read `docs/` DESIGN and PLAN documents before making changes. Current focus is the v2 headless direct control approach (`DESIGN-V2-HEADLESS.md`).

## Rules

- Respond in Simplified Chinese
- Use `just` instead of `make` for build commands
- Do not add features not listed in design documents
- Hidden API reflection calls must have try-catch + fallback
- v2 changes must not alter tool interfaces; only swap internal implementation paths, keeping v1 as fallback
