use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Action, Card, Pot};

pub type ChipCount = u64;
pub type Seat = u8; // 0 or 1

pub fn seat_is_valid(seat: Seat) -> bool {
    seat == 0 || seat == 1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Street {
    PreFlop,
    Flop,
    Turn,
    River,
    Showdown,
    Finished,
}

/// Represents a single poker hand (one deal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hand {
    pub id: HandId,
    pub deck: super::Deck,
    pub hole_cards: [Vec<Card>; 2], // index = seat
    pub community_cards: Vec<Card>,
    pub pot: Pot,
    pub current_street: Street,
    pub actions: Vec<Action>,
    pub player_stacks: [ChipCount; 2],
    pub button_position: Seat, // 0 or 1
    pub last_action_time: Option<DateTime<Utc>>,
}

impl Hand {
    pub fn deal() -> Self {
        todo!()
    }

    pub fn advance_street(&mut self) {
        todo!()
    }

    pub fn apply_action(&mut self, _action: Action) -> Result<(), String> {
        todo!()
    }

    pub fn evaluate_winner(&self) -> Vec<Seat> {
        todo!()
    }

    pub fn validate(&self) -> Result<(), String> {
        if !seat_is_valid(self.button_position) {
            return Err(format!("invalid button position {}", self.button_position));
        }
        // hole_cards must have exactly 2 cards per seat
        for (i, cards) in self.hole_cards.iter().enumerate() {
            if cards.len() != 2 {
                return Err(format!("hole_cards[{}] length {} != 2", i, cards.len()));
            }
        }
        // community_cards length must match street
        let expected_community = match self.current_street {
            Street::PreFlop => 0,
            Street::Flop => 3,
            Street::Turn => 4,
            Street::River => 5,
            Street::Showdown | Street::Finished => 5,
        };
        if self.community_cards.len() != expected_community {
            return Err(format!("community_cards length {} does not match street {:?} (expected {})", self.community_cards.len(), self.current_street, expected_community));
        }
        // pot validation
        self.pot.validate()?;
        // validate each action
        for (i, action) in self.actions.iter().enumerate() {
            action.validate()
                .map_err(|e| format!("action {} invalid: {}", i, e))?;
        }
        // chip conservation: sum of player_stacks + pot.total() must equal initial total (but we don't have initial total)
        // For now, just ensure player_stacks are non-negative
        for (i, &stack) in self.player_stacks.iter().enumerate() {
            // stacks can be zero
        }
        Ok(())
    }
}