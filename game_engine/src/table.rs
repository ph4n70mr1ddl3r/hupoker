use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Hand, Player, Seat};

pub type ChipCount = u64;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableId(String); // e.g., "table-1"

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableConfig {
    pub small_blind: ChipCount,
    pub big_blind: ChipCount,
    pub starting_stack: ChipCount,
    pub action_timeout_secs: u64,
    pub reconnection_timeout_secs: u64,
}

impl TableConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.small_blind >= self.big_blind {
            return Err(format!("small_blind ({}) must be less than big_blind ({})", self.small_blind, self.big_blind));
        }
        if self.starting_stack < self.big_blind * 20 {
            return Err(format!("starting_stack ({}) must be at least big_blind * 20 ({})", self.starting_stack, self.big_blind * 20));
        }
        if self.action_timeout_secs == 0 {
            return Err("action_timeout_secs must be at least 1".to_string());
        }
        if self.reconnection_timeout_secs < self.action_timeout_secs {
            return Err(format!("reconnection_timeout_secs ({}) must be >= action_timeout_secs ({})", self.reconnection_timeout_secs, self.action_timeout_secs));
        }
        Ok(())
    }
}

/// A heads‑up poker table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub id: TableId,
    pub seats: [Option<Player>; 2],
    pub current_hand: Option<Hand>,
    pub config: TableConfig,
    pub created_at: DateTime<Utc>,
}

impl Table {
    pub fn validate(&self) -> Result<(), String> {
        self.config.validate()?;
        // Ensure at most one player per seat
        for (i, seat) in self.seats.iter().enumerate() {
            if let Some(player) = seat {
                if player.seat as usize != i {
                    return Err(format!("player seat {} does not match index {}", player.seat, i));
                }
                // Additional player validation could go here
            }
        }
        // If current_hand is Some, both seats must be occupied and not sitting out
        if let Some(hand) = &self.current_hand {
            let occupied_seats: Vec<_> = self.seats.iter().filter_map(|s| s.as_ref()).collect();
            if occupied_seats.len() != 2 {
                return Err("current_hand exists but not both seats occupied".to_string());
            }
            // TODO: check sitting out status
        }
        Ok(())
    }
}