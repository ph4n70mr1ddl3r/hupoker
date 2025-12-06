# Network Protocol Contract

**Version**: 1.0  
**Date**: 2025‑12‑06  
**Purpose**: Define the JSON‑over‑TCP message format for communication between the server and the Windows desktop client. This contract is the single source of truth for the network layer; both sides must implement exactly these messages.

## 1. Overview

- **Transport**: Plain TCP (no TLS for MVP). Connections are persistent; the client stays connected for the duration of a session.
- **Framing**: Each message is a single UTF‑8 JSON object followed by a newline (`\n`). The newline acts as a delimiter.
- **Encoding**: JSON only; no binary payloads.
- **Versioning**: Each message includes a `"version"` field with the format `"major.minor"`. The server accepts messages with the same `major` version and `minor` ≥ client’s `minor`. The server never sends a message with a `major` version higher than the client’s supported `major`. Breaking changes increment `major`.
- **Connection Lifecycle**:
  1. Client opens TCP connection to server address (from config).
  2. Client sends `ClientHello`.
  3. Server responds with `ServerHello` (accepts/rejects).
  4. After acceptance, client may send `JoinTable`.
  5. Server streams `TableState` updates and `HandState` updates.
  6. Client sends `Action` when it’s the player’s turn.
  7. On disconnect, server reserves seat for `reconnection_timeout_secs`.

## 2. Common Structures

These types appear inside messages and are defined in the data model (`data‑model.md`). They are repeated here for clarity.

### Card
```json
{
  "rank": "Ace",
  "suit": "Spades"
}
```

`rank` ∈ `["Two","Three","Four","Five","Six","Seven","Eight","Nine","Ten","Jack","Queen","King","Ace"]`  
`suit` ∈ `["Clubs","Diamonds","Hearts","Spades"]`

### Seat
Integer `0` or `1`.

### ChipCount
Unsigned integer (`u64`), representing number of chips.

### ActionKind
String ∈ `["Fold","Check","Call","Bet","Raise"]`.

### Street
String ∈ `["PreFlop","Flop","Turn","River","Showdown","Finished"]`.

## 3. Client‑to‑Server Messages

### `ClientHello`
Sent immediately after TCP connection is established. Announces client capabilities and supported protocol version.

```json
{
  "type": "ClientHello",
  "version": "1.0",
  "client_name": "hupoker‑desktop‑windows",
  "client_version": "0.1.0"
}
```

**Fields**:
- `type`: **must** be `"ClientHello"`.
- `version`: protocol version the client understands (format `"major.minor"`).
- `client_name`: free‑form identifier for logging.
- `client_version`: semantic version of the client software.

**Response**: Server replies with `ServerHello`.

### `JoinTable`
Request to occupy a seat at a specific table. If the seat is empty (or reserved but within reconnection timeout), the server assigns the seat to this connection.

```json
{
  "type": "JoinTable",
  "table_id": "table-1",
  "seat": 0
}
```

**Fields**:
- `type`: `"JoinTable"`.
- `table_id`: string identifier of the table (as defined in server config).
- `seat`: `0` or `1` – which seat the player wishes to take.

**Response**:
- If successful, server sends a `TableState` reflecting the updated seat occupation.
- If the seat is already occupied by a connected player, server sends `Error` with `"seat_taken"`.
- If the seat is reserved (disconnected player within timeout), server sends `Error` with `"seat_reserved"`.
- If the table does not exist, `Error` with `"table_not_found"`.

### `Action`
Player’s decision when it’s their turn.

```json
{
  "type": "Action",
  "hand_id": "550e8400-e29b-41d4-a716-446655440000",
  "kind": "Raise",
  "amount": 200
}
```

**Fields**:
- `type`: `"Action"`.
- `hand_id`: UUID of the hand this action belongs to (as provided by the server in `HandState`).
- `kind`: one of `ActionKind` strings.
- `amount`: required for `Call`, `Bet`, `Raise`; must be omitted (or `null`) for `Fold` and `Check`. Must be ≥ minimum raise (big blind) for `Bet`/`Raise`, unless it equals the player’s remaining stack (all‑in).

**Response**:
- If valid, server processes the action and broadcasts updated `HandState` (and possibly `TableState`).
- If invalid (wrong turn, illegal amount, etc.), server sends `Error` with details.

### `Heartbeat`
Optional keep‑alive; can be sent by either side. Server may disconnect silent clients after a prolonged period (e.g., 120 seconds).

```json
{
  "type": "Heartbeat",
  "timestamp": "2025‑12‑06T14:30:00Z"
}
```

**Response**: The other side should reply with a `Heartbeat` containing its own timestamp (echo not required).

## 4. Server‑to‑Client Messages

### `ServerHello`
Reply to `ClientHello`. Accepts or rejects the connection based on version compatibility.

```json
{
  "type": "ServerHello",
  "version": "1.0",
  "status": "accepted",
  "server_name": "hupoker‑server",
  "server_version": "0.1.0"
}
```

**Fields**:
- `type`: `"ServerHello"`.
- `version`: the protocol version the server will use for this session (same `major` as client, `minor` ≥ client’s `minor`).
- `status`: `"accepted"` or `"rejected"`. If `"rejected"`, the connection will be closed after this message.
- `server_name` / `server_version`: informational.

**Rejection reasons**:
- Client’s `major` version is higher than server supports → `"unsupported_version"`.
- Client’s `major` version is lower than server’s minimum → `"version_too_old"`.
- Server is at capacity → `"server_full"`.

### `TableState`
Sent when a table’s metadata changes: seat occupation, player stacks, table config, etc. Also sent as initial response to `JoinTable`.

```json
{
  "type": "TableState",
  "table_id": "table-1",
  "seats": [
    {
      "seat": 0,
      "player": {
        "seat": 0,
        "stack": 1500,
        "connection_id": "123e4567-e89b-12d3-a456-426614174000",
        "disconnected_at": null,
        "is_sitting_out": false
      }
    },
    {
      "seat": 1,
      "player": null
    }
  ],
  "config": {
    "small_blind": 10,
    "big_blind": 20,
    "starting_stack": 1500,
    "action_timeout_secs": 30,
    "reconnection_timeout_secs": 60
  },
  "current_hand_id": null
}
```

**Fields**:
- `type`: `"TableState"`.
- `table_id`: string identifier.
- `seats`: array of two objects, each with `seat` (0/1) and `player` (full `Player` object as defined in data model, or `null` if seat empty).
- `config`: `TableConfig` object (see data model).
- `current_hand_id`: UUID of the currently active hand, or `null` if no hand in progress.

### `HandState`
Sent when a hand’s state changes: new hand started, new action applied, street advanced, hand finished. Contains the complete public state of the hand (no hole cards of opponents).

```json
{
  "type": "HandState",
  "hand_id": "550e8400-e29b-41d4-a716-446655440000",
  "table_id": "table-1",
  "hole_cards": ["Ac", "Kd"],               // only for the receiving player; otherwise empty array []
  "community_cards": ["2s", "3s", "4s"],
  "pot": {
    "main": 320,
    "side_pots": []
  },
  "current_street": "Flop",
  "actions": [
    {
      "seat": 0,
      "kind": "Bet",
      "amount": 100,
      "timestamp": "2025‑12‑06T14:30:00Z"
    }
  ],
  "player_stacks": [1400, 1500],
  "button_position": 0,
  "last_action_time": "2025‑12‑06T14:30:00Z",
  "acting_seat": 1,                         // whose turn it is now (or null if hand finished)
  "time_remaining_ms": 25000                // milliseconds left for the acting player
}
```

**Fields**:
- `type`: `"HandState"`.
- `hand_id`: unique identifier for this hand.
- `table_id`: which table this hand belongs to.
- `hole_cards`: array of two `Card` objects **only for the player who owns them**; for other players or spectators, an empty array `[]`.
- `community_cards`: array of `Card`s already on board.
- `pot`: `Pot` object (main + side pots).
- `current_street`: `Street` string.
- `actions`: chronological list of `Action` objects (all public actions so far).
- `player_stacks`: array of two `ChipCount`s (stacks at the **start** of the hand).
- `button_position`: seat index of the dealer/button.
- `last_action_time`: ISO‑8601 timestamp of the most recent action.
- `acting_seat`: seat index of the player who must act now (`null` if hand is finished or between streets).
- `time_remaining_ms`: milliseconds remaining for the acting player to decide (server‑side countdown).

### `Error`
Indicates that the previous client message was invalid or could not be processed.

```json
{
  "type": "Error",
  "code": "illegal_action",
  "message": "Raise amount 75 is less than minimum raise (100).",
  "original_type": "Action"
}
```

**Fields**:
- `type`: `"Error"`.
- `code`: machine‑readable error code (see table below).
- `message`: human‑readable description.
- `original_type`: the client message type that triggered the error.

**Common error codes**:
- `"invalid_message"`: JSON parsing failed or required fields missing.
- `"unsupported_version"`: protocol version mismatch.
- `"seat_taken"`: requested seat already occupied.
- `"seat_reserved"`: seat reserved for disconnected player.
- `"table_not_found"`: `table_id` does not exist.
- `"not_your_turn"`: action sent when it’s not the player’s turn.
- `"illegal_action"`: action violates poker rules (e.g., check when a bet is pending).
- `"hand_not_found"`: `hand_id` does not refer to an active hand.
- `"insufficient_stack"`: bet/raise amount exceeds player’s stack.
- `"invalid_amount"`: amount is missing where required or is out of valid range.

### `Disconnected`
Informs the client that its connection is being terminated (gracefully). Sent before the server closes the socket.

```json
{
  "type": "Disconnected",
  "reason": "server_shutdown"
}
```

**Reasons**:
- `"server_shutdown"`: server is stopping.
- `"idle_timeout"`: client sent no messages for too long.
- `"version_deprecated"`: client version is no longer supported.
- `"administrative"`: manual intervention.

## 5. Sequence Examples

### Successful Connection and Join
```
Client → Server: {"type":"ClientHello","version":"1.0","client_name":"hupoker‑desktop","client_version":"0.1.0"}
Server → Client: {"type":"ServerHello","version":"1.0","status":"accepted","server_name":"hupoker‑server","server_version":"0.1.0"}
Client → Server: {"type":"JoinTable","table_id":"table-1","seat":0}
Server → Client: {"type":"TableState",…}
```

### Hand Play
```
Server → Client: {"type":"HandState",…,"acting_seat":0}   # it’s your turn
Client → Server: {"type":"Action","hand_id":"…","kind":"Bet","amount":100}
Server → Client: {"type":"HandState",…,"acting_seat":1}   # updated state, now opponent’s turn
```

### Reconnection Flow
1. Player disconnects (TCP drop).
2. Server marks seat as `disconnected_at = now`.
3. Within timeout, player reconnects (new TCP connection), sends `ClientHello`.
4. Server accepts with `ServerHello`.
5. Client sends `JoinTable` with same `table_id` and `seat`.
6. Server recognizes the seat is reserved for that player (by seat reservation logic) and allows re‑join.
7. Server sends current `TableState` and, if a hand is active, `HandState` (with the player’s hole cards).

## 6. Versioning Policy

- **Major version (`X` in `X.y`)** increments when a change breaks compatibility: existing clients that only support version `X.y` cannot communicate with a server using `X+1.y`. Examples: removing a required field, changing field type, renaming message type.
- **Minor version (`y` in `X.y`)** increments when a change is backward‑compatible: a client that supports `X.y` can still talk to a server using `X.y+1`. Examples: adding optional fields, adding new message types, extending enumerations.
- The server must accept any `minor` version ≥ the client’s `minor` (same `major`). The server must respond with the highest `minor` it supports (but same `major`).
- Clients should ignore unknown fields (forward compatibility).

## 7. Security Considerations (MVP)

- No encryption of wire data (plain JSON over TCP).
- No authentication of players (anonymous seats).
- Server validates all actions; client is untrusted.
- RNG seeds are encrypted in audit log, not transmitted.
- Connection‑level denial‑of‑service is out of scope (assume trusted LAN environment).

---

**Signed off**: Phase 1 Design – Protocol Contract v1.0