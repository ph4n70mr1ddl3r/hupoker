use game_engine::{Action, ActionKind, Card, HandId, Player, Pot, Street, TableConfig, TableId};
use serde::{Deserialize, Serialize};

/// All possible messages (client‑to‑server and server‑to‑client).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    // Client-to-server messages
    ClientHello {
        version: String,
        client_name: String,
        client_version: String,
    },
    JoinTable {
        version: String,
        table_id: TableId,
        seat: u8,
    },
    Action {
        version: String,
        hand_id: HandId,
        kind: ActionKind, // "Fold", "Check", "Call", "Bet", "Raise"
        amount: Option<u64>,
    },
    Heartbeat {
        version: String,
        timestamp: String, // ISO 8601
    },
    // Server-to-client messages
    ServerHello {
        version: String,
        status: String, // "accepted" or "rejected"
        server_name: String,
        server_version: String,
    },
    TableState {
        version: String,
        table_id: TableId,
        seats: Vec<TableSeat>,
        config: TableConfig,
        current_hand_id: Option<HandId>,
    },
    HandState {
        version: String,
        hand_id: HandId,
        table_id: TableId,
        hole_cards: Vec<Card>, // only for the receiving player
        community_cards: Vec<Card>,
        pot: Pot,
        current_street: Street,
        actions: Vec<Action>,
        player_stacks: [u64; 2],
        button_position: u8,
        last_action_time: String, // ISO 8601
        acting_seat: Option<u8>,
        time_remaining_ms: u64,
    },
    Error {
        version: String,
        code: String,
        message: String,
        original_type: String,
    },
    Disconnected {
        version: String,
        reason: String,
    },
}

/// Seat entry in TableState.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSeat {
    pub seat: u8,
    pub player: Option<Player>,
}

// Legacy structs kept for convenience (not used in serialization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientHello {
    pub client_name: String,
    pub client_version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinTable {
    pub table_id: TableId,
    pub seat: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMessage {
    pub hand_id: HandId,
    pub kind: ActionKind,
    pub amount: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHello {
    pub status: String,
    pub server_name: String,
    pub server_version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableState {
    pub table_id: TableId,
    pub seats: Vec<TableSeat>,
    pub config: TableConfig,
    pub current_hand_id: Option<HandId>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandState {
    pub hand_id: HandId,
    pub table_id: TableId,
    pub hole_cards: Vec<Card>,
    pub community_cards: Vec<Card>,
    pub pot: Pot,
    pub current_street: Street,
    pub actions: Vec<Action>,
    pub player_stacks: [u64; 2],
    pub button_position: u8,
    pub last_action_time: String,
    pub acting_seat: Option<u8>,
    pub time_remaining_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub code: String,
    pub message: String,
    pub original_type: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disconnected {
    pub reason: String,
}
