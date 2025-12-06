use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Seat;

pub type ChipCount = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(Uuid);

/// A connected player at a table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub seat: Seat,
    pub stack: ChipCount,
    pub connection_id: ConnectionId, // ephemeral, not persisted
    pub disconnected_at: Option<DateTime<Utc>>,
    pub is_sitting_out: bool,
}

impl Player {
    pub fn validate(&self) -> Result<(), String> {
        if !super::hand::seat_is_valid(self.seat) {
            return Err(format!("invalid seat {}", self.seat));
        }
        // stack can be zero (busted)
        // No validation for connection_id
        // disconnected_at can be None or some past timestamp
        Ok(())
    }
}