# Implementation Plan: Heads-up NLHE Poker MVP (Rust Server + Windows Desktop Client)

**Branch**: `001-heads-up-poker-mvp` | **Date**: 2025-12-06 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-heads-up-poker-mvp/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Build a minimal but robust heads‑up No‑Limit Texas Hold’em poker system with:

* An authoritative Rust backend server that manages game state, shuffling, and networking.
* A Rust‑based Windows desktop client that connects to the server, displays the table, and lets a player act.
* Basic handling of disconnections, timeouts, and table configuration via a config file.
* Architecture that can be extended later (more tables, game modes, different clients) without breaking core principles.

**Technical approach**: Separate the system into three Rust crates: `game_engine` (pure logic), `server` (network layer, table management), `client_desktop` (Windows UI). Use JSON over TCP for the network protocol with semantic versioning. Employ a cryptographically secure RNG for shuffling, store seeds encrypted in a separate audit log. Configuration via TOML file.

## Technical Context

**Language/Version**: Rust 1.78 or later (stable channel)  
**Primary Dependencies**: `rand_core` + `getrandom` + `rand_chacha` (RNG), `tokio` (async runtime), `serde` + `serde_json` (serialization), `egui` + `eframe` (UI), `tracing` (logging), `thiserror`/`anyhow` (errors), `toml` (config), `chacha20poly1305` (encryption).  
**Storage**: Configuration files (TOML) + separate encrypted audit log (JSONL) for RNG seeds; no persistent database for MVP.  
**Testing**: `cargo test` with unit tests for game engine, integration tests for full hand simulation, contract tests for network protocol.  
**Target Platform**: Server: Linux (including WSL); Client: Windows 10+ (64‑bit).  
**Project Type**: Single Rust workspace with three crates (game_engine, server, client_desktop).  
**Performance Goals**: Server action processing <50 ms p95; client UI 60 fps idle / 30 fps animations; network round‑trip <200 ms acceptable.  
**Constraints**: Rust‑only per constitution; cryptographically secure RNG; deterministic game logic; no client‑side business logic; minimal dependencies (MIT/Apache‑2.0/BSD licenses).  
**Scale/Scope**: Heads‑up tables only (2 players per table); configurable number of tables via config; no authentication/accounts; no spectator clients.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**✅ Tech Stack**
- [x] Server logic implemented in Rust
- [x] Desktop client implemented in Rust with Rust‑native UI (no Electron/Node)
- [x] Server runs on Linux (including WSL)
- [x] Protocol well‑specified and versioned (planned: JSON over TCP with semantic versioning)
- [x] Minimal, well‑maintained dependencies (to be validated during research)
- [x] No hidden network calls to third‑party SaaS for core logic/RNG

**✅ Game Rules & Fairness**
- [x] Heads‑up NLHE only (per spec)
- [x] Server is single source of truth (FR‑003)
- [x] Cryptographically secure RNG for shuffling (FR‑002)
- [x] Deterministic state transitions (FR‑002)
- [x] Audit logging without exposing hole cards (FR‑008)
- [x] No admin peek at hidden cards during live play

**✅ Networking & Reliability**
- [x] Server authoritative, validates all client actions (FR‑003)
- [x] Handles transient disconnections with reconnection timeout (FR‑006)
- [x] Handles timeouts with auto‑fold (FR‑007)
- [x] Reconnection/timeout rules described in spec (clarified)
- [x] Protocol backwards‑compatible across minor versions (semantic versioning)

**✅ Architecture & Code Organization**
- [x] Clear separation: game engine, networking, persistence/logging, client UI (FR‑013)
- [x] Game engine pure Rust logic, minimal external dependencies
- [x] No global mutable state (dependency injection preferred)

**✅ Security & Privacy**
- [x] All external inputs validated (spec‑implied)
- [x] No logging of secrets/RNG seeds (encrypted audit log)
- [x] Client treated as untrusted (server validates all actions)

**✅ UX & Player Experience**
- [x] Client responsive under normal network conditions
- [x] Clear feedback: current action, time remaining, connection status (FR‑010)
- [x] Accessibility considered (color‑blind friendly design, readable fonts)

**✅ Quality Practices**
- [x] Rust code passes `rustfmt` and `clippy` (FR‑014)
- [x] Core game engine and networking covered by automated tests (FR‑012)
- [x] Follow spec‑plan‑tasks pipeline (in progress)

**⚠️ Open validation items (to be confirmed during research)**
- Exact RNG crate/OS primitive selection
- UI library compatibility with Windows and Rust‑only constraint
- Dependency license compliance (MIT/Apache‑2.0/BSD)

**Result**: All constitution gates pass for MVP scope; remaining validation items are research tasks.

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
hupoker/
├── Cargo.toml                    # Workspace definition
├── game_engine/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── deck.rs
│       ├── hand_evaluation.rs
│       ├── game_state.rs
│       ├── betting.rs
│       └── rng.rs
├── server/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── table_manager.rs
│       ├── session.rs
│       ├── protocol/
│       │   ├── mod.rs
│       │   ├── messages.rs
│       │   └── codec.rs
│       └── audit_log.rs
├── client_desktop/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── connection.rs
│       ├── ui/
│       │   ├── mod.rs
│       │   ├── table_view.rs
│       │   ├── cards.rs
│       │   └── chips.rs
│       └── config.rs
└── tests/
    ├── contract/
    │   └── protocol_tests.rs
    ├── integration/
    │   └── full_hand_simulation.rs
    └── unit/
        └── game_engine_tests.rs
```

**Structure Decision**: Single Rust workspace with three crates (`game_engine`, `server`, `client_desktop`) plus a shared `tests` directory at workspace root. This aligns with the constitution's separation of concerns and allows independent development and testing of each component.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
