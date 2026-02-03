use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use super::{Hand, HandId, Player, Seat};

pub type ChipCount = u64;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableId(String); // e.g., "table-1"

impl TableId {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
            return Err(format!(
                "small_blind ({}) must be less than big_blind ({})",
                self.small_blind, self.big_blind
            ));
        }
        if self.starting_stack < self.big_blind * 20 {
            return Err(format!(
                "starting_stack ({}) must be at least big_blind * 20 ({})",
                self.starting_stack,
                self.big_blind * 20
            ));
        }
        if self.action_timeout_secs == 0 {
            return Err("action_timeout_secs must be at least 1".to_string());
        }
        if self.reconnection_timeout_secs < self.action_timeout_secs {
            return Err(format!(
                "reconnection_timeout_secs ({}) must be >= action_timeout_secs ({})",
                self.reconnection_timeout_secs, self.action_timeout_secs
            ));
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
    pub next_button_position: Seat,
    pub hand_count: u64,
    pub config: TableConfig,
    pub created_at: DateTime<Utc>,
}

impl Table {
    pub fn validate(&self) -> Result<(), String> {
        self.config.validate()?;
        // Validate next_button_position
        if !super::hand::seat_is_valid(self.next_button_position) {
            return Err(format!("invalid next_button_position {}", self.next_button_position));
        }
        // hand_count can be any non-negative integer (u64)
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
        if let Some(_hand) = &self.current_hand {
            let occupied_seats: Vec<_> = self.seats.iter().filter_map(|s| s.as_ref()).collect();
            if occupied_seats.len() != 2 {
                return Err("current_hand exists but not both seats occupied".to_string());
            }
            for (i, seat) in self.seats.iter().enumerate() {
                if let Some(player) = seat {
                    if player.is_sitting_out {
                        return Err(format!("seat {} is sitting out but hand is in progress", i));
                    }
                }
            }
        }
        Ok(())
    }

    /// Start a new hand at this table.
    /// Requires both seats occupied and no current hand.
    /// `seed` is a 32-byte random seed for the deck.
    /// Returns the new HandId on success.
    pub fn start_hand(&mut self, seed: [u8; 32]) -> Result<HandId, String> {
        // Validate both seats occupied
        let occupied_seats: Vec<_> = self.seats.iter().filter_map(|s| s.as_ref()).collect();
        if occupied_seats.len() != 2 {
            return Err("cannot start hand: both seats must be occupied".to_string());
        }
        // Ensure no current hand
        if self.current_hand.is_some() {
            return Err("cannot start hand: a hand is already in progress".to_string());
        }
        // Collect player stacks
        let player_stacks =
            [self.seats[0].as_ref().unwrap().stack, self.seats[1].as_ref().unwrap().stack];
        // Determine button position
        let button_position = self.next_button_position;
        // Create hand
        let hand = Hand::deal(
            self.config.small_blind,
            self.config.big_blind,
            button_position,
            player_stacks,
            seed,
        );
        // Store hand id for returning
        let hand_id = hand.id;
        // Update table state
        self.current_hand = Some(hand);
        // Advance button position for next hand (toggle 0<->1)
        self.next_button_position = 1 - self.next_button_position;
        self.hand_count += 1;
        Ok(hand_id)
    }
}
