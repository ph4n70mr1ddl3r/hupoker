# Feature Specification: Heads-up NLHE Poker MVP (Rust Server + Windows Desktop Client)

**Feature Branch**: `001-heads-up-poker-mvp`  
**Created**: 2025-12-06  
**Status**: Draft  
**Input**: User description: "Build a minimal but robust heads-up No-Limit Texas Hold'em poker system with authoritative Rust backend server and Windows desktop client, handling disconnections, timeouts, and table configuration."

## Clarifications

### Session 2025-12-06

- Q: How are players identified for reconnection? → A: Seat‑based identification – player is identified by the seat position they occupy; the seat is reserved during the timeout window.
- Q: What are reasonable default timeout values? → A: Action timeout 30 seconds, reconnection timeout 60 seconds.
- Q: How are RNG seeds stored for auditability? → A: Store seed in separate secured audit log, encrypted.
- Q: What specific NLHE betting rules apply? → A: Follow standard NLHE rules with typical defaults (minimum raise = big blind, unlimited raises heads‑up, all‑in allowed).
- Q: What protocol versioning scheme is used? → A: Major.minor semantic versioning (breaking changes increment major, compatible extensions increment minor).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Connect and Join Table (Priority: P1)

As a player, I can launch the Windows client, enter the server address, and join a heads-up table configured on the server.

**Why this priority**: Without connection and table joining, no gameplay can occur. This is the foundational user journey for accessing the poker system.

**Independent Test**: Can be tested by launching the client, connecting to a running server, and successfully joining an available table. This delivers the ability to participate in a poker game.

**Acceptance Scenarios**:

1. **Given** the server is running with a configured table, **When** a player launches the client and provides the correct server address, **Then** the client connects and displays available tables.
2. **Given** the client is connected to the server, **When** the player selects a table, **Then** the player joins the table and sees the current table state (player positions, stacks, blinds).

---

### User Story 2 - Play Full NLHE Hand (Priority: P1)

As a player, I can play a full NLHE hand (from blinds to showdown/fold), seeing all actions in order.

**Why this priority**: Core gameplay is essential for MVP. This story delivers the primary value proposition of playing poker.

**Independent Test**: Can be tested by two clients joining a table and completing a full hand through all betting streets (preflop, flop, turn, river) to showdown or fold. This delivers a complete poker hand experience.

**Acceptance Scenarios**:

1. **Given** two players have joined a table, **When** the hand begins, **Then** blinds are posted and hole cards are dealt (visible only to each player).
2. **Given** it is a player's turn, **When** the player chooses a legal action (fold, check, call, bet, raise), **Then** the action is processed and the game state updates for both players.
3. **Given** all betting rounds are complete, **When** the hand reaches showdown, **Then** the winner is determined based on standard NLHE hand rankings and chips are awarded.

---

### User Story 3 - Reconnect After Temporary Disconnection (Priority: P2)

As a player, if my internet blips for a few seconds, I can reconnect and resume my seat as long as I'm within the reconnection timeout.

**Why this priority**: Network reliability is important for user experience, but reconnection can be added after core gameplay works.

**Independent Test**: Can be tested by disconnecting a client during a hand, reconnecting within the timeout, and verifying the player can resume play without losing their seat or chips.

**Acceptance Scenarios**:

1. **Given** a player is disconnected during a hand, **When** the player reconnects within the configured timeout, **Then** the player resumes their seat with current stack and hand state.
2. **Given** a player is disconnected, **When** the player fails to reconnect within the timeout, **Then** the player's hand is folded and their seat becomes available.

---

### User Story 4 - Auto-fold/Sit-out on Timeout (Priority: P2)

As a player, if I do not act within the allowed time, the game will auto-fold or sit me out in a predictable and documented way.

**Why this priority**: Game flow must continue despite player inaction, but this can be implemented after basic turn handling works.

**Independent Test**: Can be tested by having a player exceed the action timeout and verifying the system automatically folds their hand or marks them as sitting out.

**Acceptance Scenarios**:

1. **Given** it is a player's turn to act, **When** the player does not act within the configured time limit, **Then** the system automatically folds the player's hand.
2. **Given** a player is sitting out, **When** the player returns and indicates readiness, **Then** the player can resume playing in subsequent hands.

---

### User Story 5 - Run Game Engine Tests (Priority: P3)

As a developer, I can run a test suite that verifies core hand evaluation and state transitions from the game engine crate without any networking.

**Why this priority**: Testability is important for code quality but can follow core functionality.

**Independent Test**: Can be tested by running the game engine test suite and verifying all tests pass, demonstrating correct hand evaluation and state machine behavior.

**Acceptance Scenarios**:

1. **Given** the game engine crate is built, **When** `cargo test` is executed, **Then** all unit and integration tests pass, including hand evaluation and state transition tests.

---

### User Story 6 - Audit Game Logs (Priority: P3)

As a developer, I can read the server logs to understand how a particular hand played out after the fact.

**Why this priority**: Auditability supports debugging and fairness verification but is not required for MVP gameplay.

**Independent Test**: Can be tested by playing a hand and verifying the server logs contain all key events (shuffles, deals, actions, outcomes) without exposing secret information like hole cards of active players.

**Acceptance Scenarios**:

1. **Given** a hand has been played, **When** examining server logs, **Then** all player actions, community cards, and outcomes are recorded for audit purposes.
2. **Given** a hand is in progress, **When** examining server logs, **Then** hole cards of active players are not exposed until after the hand completes.

### Edge Cases

- What happens when a player attempts to bet more chips than they have? (All-in handling)
- How does the system handle a player disconnecting during a betting round?
- What occurs when the server shuts down during a hand? (Hand is canceled, chips returned)
- How does the system handle invalid messages from clients? (Reject with error, maintain state)
- What happens when a player tries to join a full table? (Reject with "table full" message)
- How does the system handle clock skew between server and client? (Server time authoritative)
- What occurs when network latency causes delayed actions? (Server enforces turn order, rejects late actions)
- How does the system handle a player attempting to act out of turn? (Reject action)
- What happens when a player's bet size is not a legal increment? (Reject, specify valid amounts)
- How does the system handle a player reconnecting with a different client instance? (Reconnect to reserved seat if within timeout)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST implement a heads-up No-Limit Texas Hold'em game engine with standard 52-card deck, hand evaluation, and betting rules.
- **FR-002**: System MUST manage game state transitions deterministically based on player actions and cryptographically secure RNG for shuffling.
- **FR-003**: Server MUST be the single source of truth for all game state, validating all client actions.
- **FR-004**: Server MUST support configurable tables via a configuration file defining blinds, starting stacks, time limits, and reconnection timeouts.
- **FR-005**: Server MUST implement a network protocol for client communication with versioned messages.
- **FR-006**: Server MUST handle player disconnections by allowing reconnection within a configurable timeout, otherwise auto-folding the player's hand.
- **FR-007**: Server MUST enforce turn timers and automatically fold hands when players exceed the action time limit.
- **FR-008**: Server MUST log all key game events (shuffles, deals, actions, outcomes) for audit without exposing hole cards of active players.
- **FR-009**: Windows desktop client MUST connect to the server, display table state (stacks, pot, community cards, hole cards), and allow legal actions.
- **FR-010**: Client MUST provide visual feedback for connection status, turn indication, and time remaining.
- **FR-011**: Client MUST handle network errors by attempting automatic reconnection up to 3 times with exponential backoff (1s, 2s, 4s), displaying user‑friendly error messages, and showing a persistent visual reconnection indicator while disconnected.
- **FR-012**: Game engine component MUST have unit tests for hand evaluation and state transitions, and an integration test simulating a full hand.
- **FR-013**: Project MUST implement the three‑crate workspace structure from plan.md, with separate `Cargo.toml` files for each crate, no shared business‑logic types beyond serialization structs, and independent compilation (each crate can be built and tested separately).
- **FR-014**: All code MUST pass formatting and linting checks.

### Key Entities *(include if feature involves data)*

- **Table**: Represents a heads-up poker table with two seats, blinds, starting stack, time limits, and reconnection timeout.
- **Player**: Represents a connected player with a seat position, chip stack, current hand (hole cards), and connection status.
- **Hand**: Represents a single poker hand including deck state, community cards, pot size, current betting round, and player actions.
- **Card**: Represents a standard playing card with rank and suit.
- **Deck**: Represents a shuffled 52-card deck from which cards are dealt.
- **Pot**: Represents the total chips in the pot, including side pots if applicable.
- **Action**: Represents a player's decision (fold, check, call, bet, raise) with associated amount.
- **Configuration**: Represents server configuration including table definitions, network settings, and timeouts.

### Assumptions

- The network protocol will use JSON over TCP for simplicity and debuggability.
- The desktop client will use a Rust-native UI library (e.g., egui, iced) compatible with Windows.
- Players will be automatically assigned to seats upon joining a table (no explicit "sit" action required).
- Spectator clients are not supported in MVP.
- The server will run on Linux (including WSL) and the client will target Windows.
- No authentication or user accounts are required for MVP (anonymous play).
- The configuration file format will be TOML for ease of editing.
- The cryptographically secure RNG will be provided by the OS or a well-audited Rust crate.
- Default timeouts: action timeout 30 seconds, reconnection timeout 60 seconds (configurable).
- RNG seeds are stored encrypted in a separate secured audit log for post‑hand fairness verification.
- Betting follows standard NLHE rules: minimum raise = big blind, unlimited raises heads‑up, all‑in allowed, no bet‑size cap beyond stack size.
- Network protocol uses major.minor semantic versioning for messages.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Two players can connect from separate clients and complete a full heads-up NLHE hand (blinds to showdown/fold) without manual intervention beyond legal actions.
- **SC-002**: The server correctly enforces game rules and prevents client-side cheating via message tampering (e.g., invalid bets, acting out of turn).
- **SC-003**: A player who disconnects and reconnects within the configured timeout (e.g., 60 seconds) resumes their seat without losing chips or hand state.
- **SC-004**: The game logic test suite passes all unit and integration tests, providing confidence in hand evaluation and state transition correctness.
- **SC-005**: Server logs contain a complete audit trail of a hand (shuffles, deals, actions, outcomes) without exposing hole cards of active players until hand completion.
