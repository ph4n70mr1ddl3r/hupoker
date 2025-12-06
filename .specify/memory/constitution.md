<!--
Sync Impact Report
- Version change: 0.0.0 (template) → 1.0.0
- Modified principles: All principles added (Fairness above all, Deterministic rules..., Security & integrity first, Spec-driven development, Simplicity over cleverness)
- Added sections: Project Identity & Scope, Tech Stack Constraints, Game Rules & Fairness Guarantees, Networking, Reliability & Disconnections, Architecture & Code Organization, Security & Privacy, UX & Player Experience, Quality Practices, AI Assistant & Spec Kit Usage, Governance
- Removed sections: None
- Templates requiring updates:
  ✅ .specify/templates/plan-template.md
  ✅ .specify/templates/spec-template.md
  ✅ .specify/templates/tasks-template.md
  ✅ .opencode/command/speckit.constitution.md
- Follow-up TODOs: Ratification date unknown (TODO(RATIFICATION_DATE))
-->
# Hupoker Constitution

## Project Identity & Scope

* This repository is for an online **heads-up No-Limit Texas Hold’em (NLHE)** poker game.
* There are two primary deliverables:

  * An **authoritative backend game server** written in Rust.
  * A **Windows desktop client** written in Rust.
* The system must be designed so additional client frontends (e.g. web, mobile) can be added later **without changing core game rules or server logic**.

## Core Principles

* **Fairness above all:** All gameplay must be provably fair, with no hidden advantages for any party (operators, admins, or specific players).
* **Deterministic rules, non-deterministic cards:** Game rules and state transitions must be deterministic; randomness is only used for shuffling and must come from a cryptographically secure source.
* **Security & integrity first:** No shortcuts that weaken security, game integrity, or player privacy are allowed, even for faster iteration.
* **Spec-driven development:** Every feature and change must be grounded in Spec Kit artifacts (constitution → spec → plan → tasks) rather than ad-hoc "vibe coding".
* **Simplicity over cleverness:** Prefer clear, simple designs and idiomatic Rust over overly generic or "clever" abstractions.

## Tech Stack Constraints

* **Language**

  * All production server logic MUST be implemented in **Rust**.
  * The Windows desktop client MUST be implemented in **Rust**, using a Rust-native UI stack (e.g., egui/eframe, Tauri, winit + custom renderer, etc.).
  * No Electron, no Node/JS runtime in production binaries.
* **Runtime & deployment**

  * Server must run on **Linux** (including WSL) as a long-lived process or service.
  * The protocol between server and client must be well-specified and versioned (e.g., custom binary or JSON over TCP/WebSocket). The exact protocol must be described in specs before implementation.
* **Dependencies**

  * Use **minimal, well-maintained** crates with permissive licenses (MIT, Apache-2.0, BSD).
  * Avoid heavy framework lock-in; prefer modular libraries over monolithic game frameworks.
  * No hidden network calls to third-party SaaS services for core game logic or RNG.

## Game Rules & Fairness Guarantees

* **Rules**

  * Game type: **Heads-up NLHE** only (two players per table) unless explicitly extended in future specs.
  * The server is the **single source of truth** for all game state (stacks, pots, bets, cards, timers).
  * All payouts, folds, and edge cases must follow standard NLHE rules as defined in the specs.
* **Randomness**

  * Card shuffling must use a cryptographically secure RNG from a well-audited Rust crate or OS primitives.
  * The shuffle algorithm and RNG usage must be deterministic with respect to a secret seed (for auditability), but **never expose seeds or internal RNG state** to clients.
* **Auditability**

  * Key game events (shuffles, deals, decisions, disconnections, and outcomes) must be logged in a way that enables **post-hoc fairness audits** without revealing other players' hole cards until the hand is complete.
  * Any "admin" tools must be read-only for active hands; no god-mode to peek at hidden cards during live play.

## Networking, Reliability & Disconnections

* The server is **authoritative** and must validate all client actions (bet size, turn order, timing).
* The system must handle:

  * **Transient disconnections**: allow reconnection within a configurable timeout.
  * **Timeouts**: if a player does not act within the allowed time, apply a clear, spec'd penalty (e.g., auto-fold, blinds handled consistently).
* All reconnection logic and timeout rules must be described in the spec and plan before implementation.
* The network protocol must be **backwards-compatible** across minor versions when possible; breaking changes must be versioned and coordinated.

## Architecture & Code Organization

* Separate concerns cleanly:

  * **Game engine**: rules, state transitions, hand evaluation, RNG usage.
  * **Networking layer**: connections, sessions, message encoding/decoding.
  * **Persistence/logging**: match history, audit logs, configuration.
  * **Client UI**: rendering, input handling, local state; no direct business logic for rules.
* The game engine should be **pure Rust logic** with minimal external dependencies so it can be reused by server and (optionally) client simulations.
* No global mutable state; prefer dependency injection and explicit ownership over singletons.

## Security & Privacy

* All external inputs (network messages, configuration files, UI events) must be validated and safely parsed.
* Never log secrets, raw credentials, or full RNG seeds.
* Any authentication/authorization mechanism must be spec'd before implementation (e.g., anonymous play vs. accounts).
* Resist client-side tampering: all critical checks must live on the server; the client is treated as untrusted.

## UX & Player Experience

* The desktop client must feel **responsive** under normal network conditions.
* Always provide clear feedback to the player:

  * Current action required (check, bet, fold, etc.).
  * Time remaining to act.
  * Connection status and reconnection attempts.
* Visual design can evolve, but **usability and clarity** are more important than flashy effects.
* Accessibility considerations (color-blind friendly chip/card design, readable fonts) must be respected where possible.

## Quality Practices

* All Rust code must:

  * Pass `rustfmt` formatting.
  * Pass `clippy` with no warnings in main crates, or documented exceptions.
* Core game engine and networking must be covered by automated tests (unit + integration). For these areas, aim for **high coverage and meaningful tests**, not just a percentage.
* For significant changes, follow this pipeline:

  1. `/speckit.specify` to define the feature or change.
  2. `/speckit.plan` to agree on architecture and approach.
  3. `/speckit.tasks` to break work into steps.
  4. Only then implement and merge.
* No feature should be merged that violates this constitution, even if the implementation "works".

## AI Assistant & Spec Kit Usage

* AI coding assistants (Copilot, Claude Code, etc.) must **adhere to this constitution** and produce artifacts that respect:

  * Rust-only backend and desktop client.
  * Constraints on dependencies, security, and fairness.
  * The separation between spec, plan, and implementation.
* When updating the constitution, the agent must **only update governance and constraints**, not silently generate or modify application code.
* If a requested feature conflicts with this constitution, the assistant must:

  * Call it out explicitly in the spec or plan.
  * Propose either an adjustment to the feature or an explicit constitution change.

## Governance

* This constitution is **versioned in Git** and may evolve, but changes must be explicit and intentional.
* Any change that weakens fairness, security, or Rust-only guarantees must be treated as a **breaking governance change** and justified in the spec.

**Version**: 1.0.0 | **Ratified**: TODO(RATIFICATION_DATE): unknown | **Last Amended**: 2025-12-06