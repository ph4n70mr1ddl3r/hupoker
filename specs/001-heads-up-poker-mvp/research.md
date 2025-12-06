# Technical Research: Heads‑up NLHE Poker MVP

**Date**: 2025‑12‑06  
**Purpose**: Resolve NEEDS CLARIFICATION items from Technical Context and select specific technologies, versions, and patterns that satisfy constitution constraints.

## 1. Rust Version & Toolchain

**Decision**: Rust 1.78 or later (stable channel).

**Rationale**:
- Latest stable provides up‑to‑date ecosystem and security fixes.
- 1.78 includes improved const‑generics and diagnostics.
- No need for nightly features; stability preferred.

**Alternatives considered**:
- Pinning to older stable (e.g., 1.75) for wider deployment compatibility – rejected because we control deployment environment and want modern crate compatibility.
- Using nightly for experimental features – rejected per simplicity principle.

## 2. Cryptographically Secure RNG

**Decision**: `rand_core` + `getrandom` for OS entropy, `rand_chacha` (ChaCha12) for deterministic seeded shuffling.

**Rationale**:
- `rand_core` is the standard trait crate; `getrandom` provides cross‑platform OS entropy.
- `rand_chacha` is a cryptographically secure stream cipher suitable for deterministic shuffling with a secret seed.
- ChaCha12 is faster than ChaCha20 while still secure for this use case.
- The combination allows us to generate a secret seed via OS entropy, then use that seed for deterministic shuffle per hand.

**Alternatives considered**:
- `rand` umbrella crate – includes all above but larger; we can depend only on needed sub‑crates.
- OS‑specific APIs (`/dev/urandom`, `BCryptGenRandom`) – less portable, more code.
- `ring` or other crypto libraries – overkill for shuffling.

## 3. Networking & Async Runtime

**Decision**: `tokio` as async runtime, `tokio::net::TcpStream` for TCP, `serde_json` for JSON serialization, `tracing` for structured logging.

**Rationale**:
- `tokio` is the de‑facto standard async runtime for Rust networking, with mature ecosystem.
- TCP provides reliable, ordered delivery; suitable for turn‑based game.
- JSON over TCP is simple to debug and sufficient for MVP; can be replaced with binary later.
- `serde_json` is the standard JSON (de)serialization library.
- `tracing` offers structured, composable logging needed for audit trails.

**Alternatives considered**:
- `async‑std` – less mature networking stack, smaller ecosystem.
- WebSockets – adds complexity; TCP is fine for dedicated client.
- Binary protocol (e.g., bincode, protobuf) – more efficient but harder to debug; can be added later.
- `log` crate – less structured; `tracing` supports spans and context better.

## 4. Desktop UI Library

**Decision**: `egui` + `eframe` for immediate‑mode GUI targeting Windows.

**Rationale**:
- Pure Rust, no external dependencies (OpenGL/Vulkan via `glow`).
- Immediate mode simplifies game‑state‑to‑UI mapping.
- Works on Windows, Linux, macOS, WebAssembly (future‑proof).
- Lightweight, quick to prototype.
- Good community support and active development.

**Alternatives considered**:
- `iced` (retained‑mode) – more complex, larger API surface.
- `druid` – less mature, smaller ecosystem.
- `slint` – requires C++ runtime, violates Rust‑only principle.
- Raw `winit` + `pixels` – too low‑level for MVP.

## 5. Performance Targets

**Decision**:
- Server action processing: < 50 ms p95 latency (including validation, state update, broadcast).
- Client UI: 60 fps during idle, 30 fps during animations.
- Network round‑trip: < 200 ms acceptable for turn‑based play.

**Rationale**:
- Poker is turn‑based; sub‑second latency is fine.
- 60 fps ensures smooth chip/card animations.
- Targets are conservative for MVP; can be tightened later.

**Alternatives considered**:
- No explicit performance targets – rejected because measurable criteria are needed for success validation.
- More aggressive targets (e.g., 10 ms server latency) – premature optimization.

## 6. Windows Version Support

**Decision**: Windows 10 (64‑bit) and later.

**Rationale**:
- Windows 10 still has >70% market share as of 2025.
- 64‑bit simplifies dependency management.
- No need to support older Windows versions (7, 8) for MVP.

**Alternatives considered**:
- Windows 11 only – too restrictive.
- Windows 7+ – adds compatibility burden, security risks.

## 7. Encryption for Audit Log

**Decision**: Use `chacha20poly1305` (from `chacha20poly1305` crate) for encrypting RNG seeds in audit log.

**Rationale**:
- Same cipher family as `rand_chacha` (consistency).
- Authenticated encryption ensures integrity.
- Lightweight, no external dependencies.
- Key management: encryption key stored in server configuration (separate from audit log), loaded from environment variable.

**Alternatives considered**:
- `aes‑gcm` – heavier, requires AES‑NI for performance.
- No encryption (rely on filesystem permissions) – violates constitution’s security principle.
- Hash‑only (store hash of seed) – prevents replay verification.

## 8. Configuration Format

**Decision**: TOML via `serde` + `toml` crate.

**Rationale**:
- Human‑editable, supports comments.
- `toml` crate is well‑maintained.
- Already assumed in spec.

**Alternatives considered**:
- JSON – no comments, harder to edit manually.
- YAML – more complex, extra dependency.
- INI – less structured.

## 9. License Compliance

**Decision**: All dependencies must be MIT, Apache‑2.0, or BSD‑2/3‑Clause licensed.

**Rationale**:
- Constitution requires permissive licenses.
- MIT/Apache‑2.0 are standard in Rust ecosystem.
- Will audit dependencies via `cargo‑deny` or `cargo‑license`.

**Alternatives considered**:
- Allow GPL‑compatible – could introduce copyleft contamination.
- No license checks – risky for distribution.

## 10. Protocol Semantic Versioning

**Decision**: Use `{ "version": "1.0", "type": "message_type", "payload": ... }` envelope.

**Rationale**:
- Explicit version field allows clients to reject incompatible messages.
- `"type"` discriminates message kind.
- JSON structure is extensible (new fields can be added in minor versions).
- Breaking changes increment major version; clients must upgrade.

**Alternatives considered**:
- Version‑less messages – impossible to evolve.
- Version in TCP header – more complex framing.

## 11. Testing Strategy

**Decision**:
- `game_engine`: unit tests for hand evaluation, state transitions.
- `server`: integration tests with in‑memory TCP clients.
- `client_desktop`: manual testing for UI; automated tests for connection logic.
- Contract tests for protocol messages (shared schema).

**Rationale**:
- Focus testing on core logic (constitution requirement).
- Integration tests verify networking without UI.
- Manual UI testing acceptable for MVP.

**Alternatives considered**:
- Full end‑to‑end automated UI tests – heavy, flaky.
- No client tests – would miss reconnection logic.

## 12. Audit Log Format

**Decision**: Structured JSONL (JSON Lines) written to a separate file with restricted permissions (0600).

**Rationale**:
- One JSON object per line, easy to parse and append.
- Includes timestamp, event type, encrypted seed (for shuffles), public game data.
- Separate file from general logs reduces risk of accidental exposure.

**Alternatives considered**:
- Binary format – harder to inspect.
- Same log as application logs – risk of leaking seeds via log aggregation.

## 13. Error Handling

**Decision**: Use `thiserror` for library error types, `anyhow` for application‑level errors.

**Rationale**:
- `thiserror` makes library errors explicit and matchable.
- `anyhow` simplifies error propagation in binaries.
- Errors are logged with `tracing`; client receives user‑friendly messages.

**Alternatives considered**:
- Custom error enums everywhere – verbose.
- `failure` crate – deprecated.

## Summary

All NEEDS CLARIFICATION items resolved. Technology choices align with constitution: Rust‑only, minimal dependencies, permissive licenses, security‑first. Next step: proceed to Phase 1 design (data model, contracts, quickstart).