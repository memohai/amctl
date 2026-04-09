# Autofish

English | [中文](./README_ZH.md)

**A deterministic Android control layer for AI agents.**

Autofish gives your agent a stable, explicit, and observable interface to Android devices. It does not plan tasks, orchestrate workflows, or make decisions — that is your agent's job. Autofish handles the last mile: seeing the screen, executing actions, verifying outcomes, and recovering from failures.

```text
human → agent → autofish → Android device
         ↑                       |
         └── observe / verify ───┘
```

## Why Autofish

Multi-agent orchestration is a rapidly evolving discipline. How agents coordinate, plan, retry, and delegate will converge on best practices and dedicated platforms — much like microservice orchestration converged on Kubernetes. Device control is a different concern and should not be entangled with it.

When planning and execution are fused, the agent loses authority over what actually happens, failures get absorbed instead of surfaced, and adopting a better orchestration strategy means rewriting the entire stack.

Autofish keeps the boundary clean: **device control is infrastructure, not application logic.** Your agent — or whatever orchestration layer emerges — keeps full authority over planning and decision-making. Autofish provides deterministic device operations, explicit failure signals, and a queryable execution record. When the orchestration story matures, Autofish stays the same.

## Design Principles

| Principle | What it means in practice |
|---|---|
| **Single responsibility** | Android observation and control. No planning, no LLM calls, no workflow engine. |
| **Agent-first interface** | The CLI and service API are shaped for machine consumption: structured output, deterministic exit codes, composable commands. |
| **Deterministic surface** | Commands do exactly what they say. No implicit retries, no "smart" heuristics behind the scenes. |
| **Closed-loop discipline** | Every action must be preceded by observation and followed by verification. The system enforces this through its memory model. |
| **Graceful degradation** | Multiple execution paths coexist. When preferred capabilities are unavailable, fallback paths keep the system operational. |
| **Observable execution** | Every action, state transition, and recovery is recorded with structured context — queryable by the agent mid-run. |

## How It Works

Autofish has two components:

**Android Service** — A foreground service running on the device, exposing an HTTP API. It handles screenshots, UI tree inspection, tap/swipe/text input execution, and on-screen overlay annotations.

**CLI (`af`)** — A Rust binary that talks to the service over HTTP. It provides the command interface, manages local tool memory (SQLite), and handles artifact storage. Distributed via npm (`@memohjs/af`).

### The Observe-Act-Verify Loop

```bash
af observe page --field screen --field refs --max-rows 80
af act tap --by ref --value @n3
af verify text-contains --text "Wi-Fi"
af memory save --app com.android.settings --topic "nav/home-to-wifi" \
  --content "Tap @n3 (Wi-Fi) from main settings page"
```

### Tool Memory

The CLI maintains a local SQLite database that tracks:

- **Events** — Every `act`, `verify`, and `recover` with page fingerprint, status, failure cause, and duration.
- **Transitions** — Automatically closed act → verify pairs with success/failure counters.
- **Recovery strategies** — Which recovery steps worked for which failure causes.
- **Agent notes** — Append-only knowledge your agent writes and queries.

## Documentation

- [Quickstart](./docs/QUICKSTART.md) — From zero to first command
- [CLI Reference](./cli/README.md) — All commands, flags, and output formats
- [Architecture](./docs/ARCHITECTURE.md) — Service internals, refs design, API contracts
- [CLI Memory](./docs/CLI_MEMORY.md) — Data model, recording rules, observation cache
- [Changelog](./CHANGELOG.md) — Release history and breaking changes

## Build from Source

Requirements: JDK 17, Android SDK API 36, Rust toolchain, `just`.

```bash
just build
just install
```