use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{ChipCount, Hand, HandError, HandId, Player, Seat};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableId(String);

impl TableId {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TableId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for TableId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err("table ID cannot be empty".to_string());
        }
        Ok(Self(s.to_string()))
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

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TableError {
    #[error("invalid table config: {0}")]
    InvalidConfig(String),
    #[error("invalid next button position: {0}")]
    InvalidButtonPosition(Seat),
    #[error("player seat {0} does not match index {1}")]
    SeatMismatch(Seat, usize),
    #[error("current hand exists but not both seats occupied")]
    HandWithoutPlayers,
    #[error("seat {0} is sitting out but hand is in progress")]
    SittingOutDuringHand(usize),
    #[error("cannot start hand: both seats must be occupied")]
    NotEnoughPlayers,
    #[error("cannot start hand: a hand is already in progress")]
    HandInProgress,
    #[error("seat {0} should be occupied")]
    SeatNotOccupied(usize),
    #[error("hand error: {0}")]
    HandError(#[from] HandError),
}

impl TableConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.small_blind >= self.big_blind {
            return Err(format!(
                "small_blind ({}) must be less than big_blind ({})",
                self.small_blind, self.big_blind
            ));
        }
        let min_stack = self.big_blind.saturating_mul(20);
        if self.starting_stack < min_stack {
            return Err(format!(
                "starting_stack ({}) must be at least big_blind * 20 ({})",
                self.starting_stack, min_stack
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
        if !super::hand::seat_is_valid(self.next_button_position) {
            return Err(format!("invalid next_button_position {}", self.next_button_position));
        }
        for (i, seat) in self.seats.iter().enumerate() {
            if let Some(player) = seat {
                if player.seat as usize != i {
                    return Err(format!("player seat {} does not match index {}", player.seat, i));
                }
            }
        }
        if let Some(_hand) = &self.current_hand {
            if self.seats.iter().filter_map(|s| s.as_ref()).count() != 2 {
                return Err("current_hand exists but not both seats occupied".to_string());
            }
            for (i, seat) in self.seats.iter().enumerate() {
                if let Some(player) = seat {
                    if player.is_sitting_out {
                        return Err(format!("seat {i} is sitting out but hand is in progress"));
                    }
                }
            }
        }
        Ok(())
    }

    #[must_use = "start_hand() returns a Result that must be handled"]
    pub fn start_hand(&mut self, seed: [u8; 32]) -> Result<HandId, TableError> {
        if self.seats.iter().filter_map(|s| s.as_ref()).count() != 2 {
            return Err(TableError::NotEnoughPlayers);
        }
        if self.current_hand.is_some() {
            return Err(TableError::HandInProgress);
        }
        let player_stacks = [
            self.seats[0].as_ref().map(|p| p.stack).ok_or(TableError::SeatNotOccupied(0))?,
            self.seats[1].as_ref().map(|p| p.stack).ok_or(TableError::SeatNotOccupied(1))?,
        ];
        let button_position = self.next_button_position;
        let hand = Hand::deal(
            self.config.small_blind,
            self.config.big_blind,
            button_position,
            player_stacks,
            seed,
        )?;
        let hand_id = hand.id;
        self.current_hand = Some(hand);
        self.next_button_position = 1 - self.next_button_position;
        self.hand_count = self.hand_count.saturating_add(1);
        Ok(hand_id)
    }

    pub fn start_hand_with_id(
        &mut self,
        seed: [u8; 32],
        hand_id: HandId,
    ) -> Result<HandId, TableError> {
        if self.seats.iter().filter_map(|s| s.as_ref()).count() != 2 {
            return Err(TableError::NotEnoughPlayers);
        }
        if self.current_hand.is_some() {
            return Err(TableError::HandInProgress);
        }
        let player_stacks = [
            self.seats[0].as_ref().map(|p| p.stack).ok_or(TableError::SeatNotOccupied(0))?,
            self.seats[1].as_ref().map(|p| p.stack).ok_or(TableError::SeatNotOccupied(1))?,
        ];
        let button_position = self.next_button_position;
        let hand = Hand::deal_with_id(
            self.config.small_blind,
            self.config.big_blind,
            button_position,
            player_stacks,
            seed,
            hand_id,
        )?;
        let hand_id = hand.id;
        self.current_hand = Some(hand);
        self.next_button_position = 1 - self.next_button_position;
        self.hand_count = self.hand_count.saturating_add(1);
        Ok(hand_id)
    }
}
