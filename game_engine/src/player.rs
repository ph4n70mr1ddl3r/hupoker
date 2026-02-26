use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ChipCount, Seat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(Uuid);

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ConnectionId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl std::str::FromStr for ConnectionId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(s).map_err(|e| format!("invalid connection ID: {}", e))?;
        Ok(Self(uuid))
    }
}

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
