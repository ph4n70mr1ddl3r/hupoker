# Data Model: Heads‑up NLHE Poker MVP

**Date**: 2025‑12‑06  
**Purpose**: Define core entities, their fields, relationships, and validation rules for the game engine and server.

All types are represented as Rust structs/enums with `serde` serialization support where needed. Validation methods are attached as `impl` blocks.

## 1. Card

```rust
/// Standard playing card (French deck).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rank {
    Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten,
    Jack, Queen, King, Ace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    Clubs, Diamonds, Hearts, Spades,
}
```

**Validation**: None beyond constructibility (ranks and suits are exhaustive).

## 2. Deck

```rust
/// A shuffled 52‑card deck.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    cards: Vec<Card>,
    rng: ChaCha12Rng, // seeded with secret from OS entropy
}
```

**Fields**:
- `cards`: remaining cards in deck order (top = first).
- `rng`: deterministic CSPRNG used for shuffling; not serialized.

**Methods**:
- `new(seed: [u8; 32]) -> Self`: creates and shuffles 52 cards using Fisher‑Yates.
- `draw(&mut self) -> Option<Card>`: removes and returns top card.
- `remaining(&self) -> usize`: number of cards left.

**Validation**:
- Shuffle must be deterministic given the same seed.
- Drawing from empty deck returns `None` (should not happen in a single hand).

## 3. Hand

```rust
/// Represents a single poker hand (one deal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hand {
    pub id: HandId,
    pub deck: Deck,
    pub hole_cards: [Vec<Card>; 2], // index = seat
    pub community_cards: Vec<Card>,
    pub pot: Pot,
    pub current_street: Street,
    pub actions: Vec<Action>,
    pub player_stacks: [ChipCount; 2],
    pub button_position: Seat, // 0 or 1
    pub last_action_time: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandId(uuid::Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Street {
    PreFlop,
    Flop,
    Turn,
    River,
    Showdown,
    Finished,
}

pub type ChipCount = u64;
pub type Seat = u8; // 0 or 1
```

**Fields**:
- `id`: unique identifier for the hand (for logging).
- `deck`: remaining cards after hole and community cards dealt.
- `hole_cards`: two‑element array, each seat’s private cards (2 cards each).
- `community_cards`: up to 5 cards (flop 3, turn 1, river 1).
- `pot`: current pot(s) (see Pot).
- `current_street`: which betting round is active.
- `actions`: chronological list of actions taken.
- `player_stacks`: chip counts for each seat before this hand started.
- `button_position`: dealer/button seat (small blind posts first).
- `last_action_time`: timestamp of last action (for timeout enforcement).

**Validation**:
- `Seat` must be 0 or 1.
- `hole_cards` each contain exactly 2 cards after dealing.
- `community_cards` length matches street (0 preflop, 3 flop, 4 turn, 5 river).
- `player_stacks` sum + pot.total() must equal starting stack sum (conservation of chips).

**State transitions**:
- `deal()` → creates deck, shuffles, deals hole cards, posts blinds.
- `advance_street()` → when betting round completes, deals community cards if needed.
- `apply_action(action)` → validates action, updates pot/stacks, records action.
- `evaluate_winner()` → at showdown, computes best hand for each remaining player, awards pot.

## 4. Pot

```rust
/// Pot(s) in the current hand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pot {
    pub main: ChipCount,
    pub side_pots: Vec<SidePot>, // only if all‑in situations
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidePot {
    pub amount: ChipCount,
    pub eligible_seats: Vec<Seat>, // players who contributed to this side pot
}
```

**Validation**:
- `main` ≥ 0.
- Side pots only exist when at least one player is all‑in and others have more chips.
- Side pot amounts must be consistent with betting actions.

## 5. Action

```rust
/// A player’s decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub seat: Seat,
    pub kind: ActionKind,
    pub amount: Option<ChipCount>, // Some for bet/raise/call, None for fold/check
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    Fold,
    Check,
    Call,
    Bet,
    Raise,
}
```

**Validation**:
- `seat` must be 0 or 1.
- `amount` must be `Some` for `Call`, `Bet`, `Raise`; `None` for `Fold`, `Check`.
- `amount` must be ≥ minimum raise (big blind) for `Bet`/`Raise`, unless all‑in.
- Action must be legal given current betting state (e.g., cannot check if a bet is pending).

## 6. Player (session‑time)

```rust
/// A connected player at a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub seat: Seat,
    pub stack: ChipCount,
    pub connection_id: ConnectionId, // ephemeral, not persisted
    pub disconnected_at: Option<Instant>,
    pub is_sitting_out: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(uuid::Uuid);
```

**Fields**:
- `seat`: occupied seat (0 or 1).
- `stack`: current chip count (updated after each hand).
- `connection_id`: unique ID for this connection session; reconnection uses same seat but new connection ID.
- `disconnected_at`: when the player disconnected; if `Some` and within timeout, seat is reserved.
- `is_sitting_out`: player opted out of next hand (e.g., after timeout).

**Validation**:
- `seat` must be unique per table.
- `stack` must be ≥ 0.
- If `disconnected_at` is `Some`, seat is temporarily reserved.

## 7. Table

```rust
/// A heads‑up poker table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub id: TableId,
    pub seats: [Option<Player>; 2],
    pub current_hand: Option<Hand>,
    pub config: TableConfig,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableId(String); // e.g., "table-1"

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableConfig {
    pub small_blind: ChipCount,
    pub big_blind: ChipCount,
    pub starting_stack: ChipCount,
    pub action_timeout_secs: u64,
    pub reconnection_timeout_secs: u64,
}
```

**Validation**:
- `small_blind` < `big_blind`.
- `starting_stack` ≥ `big_blind` * 20 (reasonable minimum).
- `action_timeout_secs` ≥ 1, `reconnection_timeout_secs` ≥ action timeout.
- `seats` can have at most one player per seat.
- `current_hand` is `Some` only when both seats are occupied and not sitting out.

## 8. Configuration (server‑level)

```rust
/// Server‑wide configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String, // e.g., "127.0.0.1:8080"
    pub tables: Vec<TableConfig>,
    pub audit_log_path: PathBuf,
    pub encryption_key_env_var: String, // name of env var holding ChaCha20‑Poly1305 key
}
```

**Validation**:
- `bind_address` must be a valid socket address.
- `tables` must have unique IDs.
- `audit_log_path` must be writable.
- `encryption_key_env_var` must be set at runtime (32‑bytes base64).

## Relationships

- **Table** contains 0‑2 **Player**s and optionally a **Hand**.
- **Hand** contains a **Deck**, **Pot**, and list of **Action**s.
- **Player** references a **Seat** (0/1) and has a **ConnectionId**.
- **ServerConfig** contains multiple **TableConfig**s.

## State Lifecycle

1. **Table idle**: `seats` may be empty/partial, `current_hand = None`.
2. **Hand start**: both seats occupied → create `Hand`, deal cards, post blinds.
3. **Betting round**: players act, `actions` recorded, pot updated.
4. **Street advance**: after betting round completes, deal community cards if needed.
5. **Showdown**: after river betting, evaluate hands, award pot, update player stacks.
6. **Hand end**: `current_hand = None`, wait for next hand (button moves).

## Validation Rules (Business Logic)

- **Bet sizing**: minimum raise = big blind; maximum raise = player’s remaining stack.
- **All‑in**: if a player bets more than opponent’s stack, side pots created.
- **Timeout**: if `last_action_time` + `action_timeout` < now, auto‑fold current player.
- **Disconnection**: seat reserved for `reconnection_timeout`; if timeout expires, player is removed and hand folded (if in hand).
- **Reconnection**: same seat, new `connection_id`, receive current game state.

## Serialization Notes

- `Deck::rng` is `#[serde(skip)]` – never serialized; only the `cards` vec is persisted.
- `Instant`/`DateTime` serialize as RFC‑3339 strings.
- `PathBuf` serializes as string.
- `uuid::Uuid` serializes as hyphenated string.