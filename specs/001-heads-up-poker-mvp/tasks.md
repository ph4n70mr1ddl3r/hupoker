# Tasks: Heads-up NLHE Poker MVP (Rust Server + Windows Desktop Client)

**Input**: Design documents from `/specs/001-heads-up-poker-mvp/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/
**Branch**: `001-heads-up-poker-mvp`
**Date**: 2025-12-06

**Tests**: The feature specification requests unit tests for game engine (FR‑012) and integration tests for full hand simulation. These are mandatory and included below.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic workspace structure as defined in plan.md.

- [X] T001 Create workspace root Cargo.toml with three members: `game_engine`, `server`, `client_desktop`
- [X] T002 [P] Initialize `game_engine` crate with dependencies from plan.md (`rand_core`, `getrandom`, `rand_chacha`, `serde`)
- [X] T003 [P] Initialize `server` crate with dependencies (`tokio`, `serde_json`, `tracing`, `thiserror`, `toml`, `chacha20poly1305`)
- [X] T004 [P] Initialize `client_desktop` crate with dependencies (`egui`, `eframe`, `serde_json`, `tracing`)
- [X] T005 [P] Configure workspace‑level linting (`rustfmt.toml`, `.clippy.toml`) and ensure `cargo fmt` / `cargo clippy` pass
- [X] T006 Create `tests/` directory at workspace root with subdirectories `contract/`, `integration/`, `unit/`

**Checkpoint**: Workspace ready, crates exist, dependencies declared, formatting/linting configured.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T007 Implement data‑model types (Card, Rank, Suit, Deck, Hand, Pot, Action, Player, Table, ServerConfig) in `game_engine/src/lib.rs` and corresponding modules (see `data‑model.md` for exact definitions)
- [X] T008 [P] Add validation methods for each type (e.g., `Seat` must be 0/1, `small_blind < big_blind`, etc.)
- [X] T009 Implement `Deck` shuffling using `rand_chacha` with deterministic seed (secret from OS entropy)
- [X] T010 Implement `Hand` state transitions (`deal`, `advance_street`, `apply_action`, `evaluate_winner`) – pure logic, no networking
- [X] T011 Implement betting logic (minimum raise, all‑in, side pots) in `game_engine/src/betting.rs`
- [X] T012 Implement hand evaluation (standard NLHE ranking) in `game_engine/src/hand_evaluation.rs`
- [X] T013 Create `server::config` module to parse TOML config (`ServerConfig`) and validate
- [X] T014 Create `server::audit_log` module with encrypted log writer (ChaCha20‑Poly1305) for RNG seeds and game events
- [X] T015 Create `server::protocol` module with message definitions from `contracts/protocol.md` (JSON serialization with `serde`)
- [X] T016 Create `server::table_manager` module that holds `Table` instances and manages seat assignment
- [X] T017 Create `client_desktop::connection` module with TCP client and message framing (newline‑delimited JSON)

**Checkpoint**: Game engine can simulate a full hand locally; server can parse config and manage tables; client can connect and send/receive messages; audit logging ready. User stories can now start in parallel.

---

## Phase 3: User Story 1 - Connect and Join Table (Priority: P1) 🎯 MVP

**Goal**: A player can launch the Windows client, connect to the server, and join a configured heads‑up table.

**Independent Test**: Launch server with config, start client, connect and join table; verify server sends `TableState` with correct seat occupation.

### Tests for User Story 1 (Mandatory per FR‑005, FR‑012)

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T018 [P] [US1] Contract test for `ClientHello`/`ServerHello` handshake in `tests/contract/protocol_tests.rs`
- [X] T019 [P] [US1] Integration test for connection and table join in `tests/integration/connect_join.rs`

### Implementation for User Story 1

- [X] T020 [P] [US1] Implement server TCP listener and connection acceptance in `server/src/main.rs`
- [X] T021 [P] [US1] Implement message framing (newline‑delimited JSON) and version negotiation in `server/src/protocol/codec.rs`
- [X] T022 [US1] Handle `ClientHello` → `ServerHello` flow in `server/src/server.rs`
- [X] T023 [US1] Handle `JoinTable` request, validate seat availability, send `TableState` response
- [X] T024 [P] [US1] Create client desktop UI skeleton with connection dialog (server address input) in `client_desktop/src/ui/mod.rs`
- [X] T025 [US1] Implement client network connection and message sending/receiving in `client_desktop/src/connection.rs`
- [X] T026 [US1] Wire UI connection dialog to network layer; on successful join, display table view with seat occupation

**Checkpoint**: User Story 1 complete. Two clients can connect to server, join a table (different seats), see table state. No gameplay yet.

---

## Phase 4: User Story 2 - Play Full NLHE Hand (Priority: P1) 🎯 MVP

**Goal**: Two players can play a full NLHE hand (blinds to showdown/fold), seeing all actions in order.

**Independent Test**: Two clients join same table; server starts hand; players act through all streets; hand ends with showdown or fold; chips awarded correctly.

### Tests for User Story 2 (Mandatory per FR‑012)

- [X] T027 [P] [US2] Unit test for `Hand` state transitions (deal, betting, street advance, showdown) in `tests/unit/game_engine_tests.rs`
- [X] T028 [P] [US2] Integration test simulating a full hand with two automated players in `tests/integration/full_hand_simulation.rs`

### Implementation for User Story 2

- [X] T029 [US2] Extend `table_manager` to start a hand when both seats are occupied (create `Hand` via game engine)
- [X] T030 [US2] Implement `HandState` message generation (include hole cards only for respective player)
- [ ] T031 [US2] Broadcast `HandState` to both clients on hand start, each action, street advance, hand end
- [ ] T032 [P] [US2] Add UI table view showing community cards, pot, player stacks, and action buttons in `client_desktop/src/ui/table_view.rs`
- [ ] T033 [P] [US2] Render card graphics (simple rectangles with rank/suit) in `client_desktop/src/ui/cards.rs`
- [ ] T034 [P] [US2] Render chip stacks and pot in `client_desktop/src/ui/chips.rs`
- [ ] T035 [US2] Implement action buttons (Fold, Check, Call, Bet, Raise) with validation (disable illegal actions)
- [ ] T036 [US2] Send `Action` message when player clicks a button; update UI after server response
- [ ] T037 [US2] Server‑side validation of `Action` messages (turn order, bet size, legality) using game engine
- [ ] T038 [US2] Update `Hand` state after valid action; broadcast new `HandState`
- [ ] T039 [US2] Handle showdown: evaluate winner via game engine, award pot, update player stacks, broadcast hand result

**Checkpoint**: User Story 2 complete. Two human players can play a full NLHE hand via the GUI. MVP gameplay achieved.

---

## Phase 5: User Story 3 - Reconnect After Temporary Disconnection (Priority: P2)

**Goal**: A player can reconnect within the timeout window and resume their seat.

**Independent Test**: Disconnect client during hand, reconnect within timeout, verify seat still occupied and hand state restored.

### Tests for User Story 3 (Optional but recommended)

- [ ] T040 [P] [US3] Integration test for reconnection flow in `tests/integration/reconnection.rs`

### Implementation for User Story 3

- [ ] T041 [US3] Add `disconnected_at: Option<Instant>` field to `Player` struct (data‑model)
- [ ] T042 [US3] Server: on TCP disconnect, mark player as disconnected, start reconnection timer
- [ ] T043 [US3] Server: allow `JoinTable` for same seat within `reconnection_timeout_secs` if `disconnected_at` is set
- [ ] T044 [US3] Server: on re‑join, send current `TableState` and `HandState` (with player’s hole cards)
- [ ] T045 [US3] Client: handle network errors gracefully, show "Reconnecting…" UI, attempt automatic reconnection
- [ ] T046 [US3] Client: restore UI state after reconnection (same seat, same hand)

**Checkpoint**: User Story 3 complete. Player can survive brief network interruption without losing seat or chips.

---

## Phase 6: User Story 4 - Auto‑fold/Sit‑out on Timeout (Priority: P2)

**Goal**: If a player does not act within the allowed time, the system auto‑folds their hand (or marks them as sitting out).

**Independent Test**: Player exceeds action timeout; server auto‑folds their hand; game continues.

### Tests for User Story 4 (Optional)

- [ ] T047 [P] [US4] Integration test for action timeout in `tests/integration/timeout.rs`

### Implementation for User Story 4

- [ ] T048 [US4] Add `last_action_time: Option<Instant>` to `Hand` (already in data‑model)
- [ ] T049 [US4] Server: check each hand for timeout on each tick (e.g., every second)
- [ ] T050 [US4] If timeout exceeded, apply auto‑fold action (call `hand.apply_action` with `ActionKind::Fold`)
- [ ] T051 [US4] Broadcast `HandState` update reflecting auto‑fold
- [ ] T052 [US4] Client: display visual countdown timer for acting player
- [ ] T053 [US4] Client: handle server‑initiated auto‑fold (show "folded by timeout" message)

**Checkpoint**: User Story 4 complete. Game flow continues even if a player stalls.

---

## Phase 7: User Story 5 - Run Game Engine Tests (Priority: P3)

**Goal**: Developer can run a test suite that verifies core hand evaluation and state transitions from the game engine crate.

**Independent Test**: `cargo test -p game_engine` passes.

### Tests for User Story 5 (Already covered in previous phases)

- [ ] T054 [P] [US5] Ensure all unit tests for game engine are written (T027) and pass

### Implementation for User Story 5

- [ ] T055 [US5] Add `#[cfg(test)]` modules to `game_engine` crate (already part of T027)
- [ ] T056 [US5] Document how to run tests in `quickstart.md`

**Checkpoint**: User Story 5 complete. Game engine is fully testable.

---

## Phase 8: User Story 6 - Audit Game Logs (Priority: P3)

**Goal**: Developer can read the server logs to understand how a particular hand played out after the fact.

**Independent Test**: Play a hand, inspect audit log (decrypted) and verify it contains RNG seed and all actions.

### Tests for User Story 6 (Optional)

- [ ] T057 [P] [US6] Unit test for audit log encryption/decryption in `tests/unit/audit_log_tests.rs`

### Implementation for User Story 6

- [ ] T058 [US6] Ensure audit log writes each hand’s RNG seed (encrypted) and all public actions (unencrypted)
- [ ] T059 [US6] Add command‑line tool or documentation for decrypting audit log (use same env var key)
- [ ] T060 [US6] Log hand ID, table ID, timestamps, and final outcome

**Checkpoint**: User Story 6 complete. Post‑hand fairness verification possible.

---

## Phase 9: Polish & Cross‑Cutting Concerns

**Purpose**: Improvements that affect multiple user stories.

- [ ] T061 [P] Update `quickstart.md` with actual build/run instructions (after implementation)
- [ ] T062 [P] Run `cargo fmt` and `cargo clippy` across workspace, fix any warnings
- [ ] T063 [P] Add documentation comments to all public types/functions
- [ ] T064 Ensure all error cases are handled gracefully (network errors, malformed messages, etc.)
- [ ] T065 Performance: verify server action processing <50 ms p95, client UI responsive
- [ ] T066 Security: validate no secrets logged, encryption key never hard‑coded
- [ ] T067 Accessibility: color‑blind friendly card designs, readable fonts
- [ ] T068 Run full test suite (`cargo test --workspace`) and ensure all tests pass

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies – can start immediately.
- **Foundational (Phase 2)**: Depends on Setup completion – **BLOCKS all user stories**.
- **User Stories (Phase 3+)**: All depend on Foundational phase completion.
  - User stories can then proceed in parallel (if staffed).
  - Recommended order: US1 → US2 (MVP), then US3, US4, US5, US6.
- **Polish (Phase 9)**: Depends on all desired user stories being complete.

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2). No dependencies on other stories.
- **User Story 2 (P2)**: Can start after Foundational (Phase 2). Depends on US1 for connection/join infrastructure.
- **User Story 3 (P2)**: Can start after US1 and US2. Builds on existing connection handling.
- **User Story 4 (P2)**: Can start after US2. Requires hand state management.
- **User Story 5 (P3)**: Can start after Foundational (game engine tests are part of Phase 2).
- **User Story 6 (P3)**: Can start after US2 (audit log needs hand events).

### Within Each User Story

- Tests (if included) MUST be written and FAIL before implementation.
- Models before services, services before UI/network.
- Story complete before moving to next priority.

### Parallel Opportunities

- All Setup tasks marked **[P]** can run in parallel.
- All Foundational tasks marked **[P]** can run in parallel (within Phase 2).
- Once Foundational phase completes, US1 and US2 can be worked on in parallel by different developers (US2 depends on US1's network layer but can proceed after T021‑T022).
- Tests for a story marked **[P]** can run in parallel.
- Different UI components (cards, chips, table view) can be developed in parallel.

---

## Implementation Strategy

### MVP First (User Stories 1 + 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL – blocks all stories)
3. Complete Phase 3: User Story 1 (Connect and Join)
4. Complete Phase 4: User Story 2 (Play Full Hand)
5. **STOP and VALIDATE**: Two human players can play a full NLHE hand via GUI. MVP achieved.

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Basic connection works
3. Add User Story 2 → Test independently → Playable hand (MVP!)
4. Add User Story 3 → Reconnection support
5. Add User Story 4 → Timeout handling
6. Add User Story 5 → Test suite
7. Add User Story 6 → Audit logs

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together.
2. Once Foundational is done:
   - Developer A: User Story 1 (server networking, client connection)
   - Developer B: User Story 2 (game engine integration, UI)
   - Developer C: User Story 3 & 4 (reconnection, timeout)
3. Stories integrate as they complete.

---

## Notes

- **[P]** tasks = different files, no dependencies.
- **[Story]** label maps task to specific user story for traceability.
- Each user story should be independently completable and testable.
- Verify tests fail before implementing.
- Commit after each task or logical group.
- Stop at any checkpoint to validate story independently.
- Avoid: vague tasks, same file conflicts, cross‑story dependencies that break independence.

**End of tasks list** – ready for implementation.