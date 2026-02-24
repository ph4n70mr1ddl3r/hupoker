use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Action, Card, ChipCount, Pot};

/// Seat index at a heads-up poker table (0 or 1).
pub type Seat = u8;

const NUM_SEATS: u8 = 2;
const HOLE_CARDS_PER_SEAT: usize = 2;

/// Returns true if the seat index is valid for a heads-up table (0 or 1).
pub fn seat_is_valid(seat: Seat) -> bool {
    seat < NUM_SEATS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandId(Uuid);

impl HandId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl std::fmt::Display for HandId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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
    #[serde(skip)]
    pub betting: super::betting::Betting,
    pub small_blind: ChipCount,
    pub big_blind: ChipCount,
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
    pub fn deal(
        small_blind: ChipCount,
        big_blind: ChipCount,
        button_position: Seat,
        player_stacks: [ChipCount; 2],
        seed: [u8; 32],
    ) -> Result<Self, String> {
        let deck = super::Deck::new(seed);
        let betting =
            super::betting::Betting::new(small_blind, big_blind, player_stacks, button_position)
                .map_err(|e| format!("failed to post blinds: {}", e))?;
        let mut deck = deck;
        let mut hole_cards = [Vec::new(), Vec::new()];
        for _ in 0..HOLE_CARDS_PER_SEAT {
            for cards in &mut hole_cards {
                if let Some(card) = deck.draw() {
                    cards.push(card);
                }
            }
        }
        let pot = super::Pot { main: betting.total_pot(), side_pots: Vec::new() };
        Ok(Self {
            id: HandId(Uuid::new_v4()),
            deck,
            betting,
            small_blind,
            big_blind,
            hole_cards,
            community_cards: Vec::new(),
            pot,
            current_street: Street::PreFlop,
            actions: Vec::new(),
            player_stacks,
            button_position,
            last_action_time: Some(Utc::now()),
        })
    }

    pub fn advance_street(&mut self) -> Result<(), String> {
        // Ensure betting round is complete
        if !self.betting.is_round_complete() {
            return Err("cannot advance street while betting round is incomplete".to_string());
        }
        // Move betting pot to main pot
        self.pot.main = self.pot.main.saturating_add(self.betting.total_pot());
        // Reset betting for next street with current stacks
        self.betting = super::betting::Betting::new_street(
            self.big_blind,
            self.betting.stacks(),
            self.button_position,
        );
        self.last_action_time = Some(Utc::now());
        // Deal community cards based on street
        match self.current_street {
            Street::PreFlop => {
                // Deal flop (3 cards)
                for _ in 0..3 {
                    if let Some(card) = self.deck.draw() {
                        self.community_cards.push(card);
                    }
                }
                self.current_street = Street::Flop;
            }
            Street::Flop => {
                // Deal turn (1 card)
                if let Some(card) = self.deck.draw() {
                    self.community_cards.push(card);
                }
                self.current_street = Street::Turn;
            }
            Street::Turn => {
                // Deal river (1 card)
                if let Some(card) = self.deck.draw() {
                    self.community_cards.push(card);
                }
                self.current_street = Street::River;
            }
            Street::River => {
                // No more cards, move to showdown
                self.current_street = Street::Showdown;
            }
            Street::Showdown => {
                self.current_street = Street::Finished;
            }
            Street::Finished => {
                return Err("hand already finished".to_string());
            }
        }
        Ok(())
    }

    pub fn apply_action(&mut self, action: Action) -> Result<(), String> {
        // Validate seat
        if !seat_is_valid(action.seat) {
            return Err(format!("invalid seat {}", action.seat));
        }
        // Apply via betting
        self.betting
            .apply_action(action.seat, action.kind, action.amount)
            .map_err(|e| e.to_string())?;
        // Record action
        self.actions.push(action);
        // Update timestamp
        self.last_action_time = Some(Utc::now());
        Ok(())
    }

    pub fn evaluate_winner(&self) -> Vec<Seat> {
        let mut folded = [false, false];
        for action in &self.actions {
            if action.kind == super::ActionKind::Fold {
                folded[action.seat as usize] = true;
            }
        }
        let active_seats: Vec<Seat> =
            (0..NUM_SEATS as usize).filter(|&i| !folded[i]).map(|i| i as Seat).collect();
        if active_seats.is_empty() {
            return Vec::new();
        }
        if active_seats.len() == 1 {
            return active_seats;
        }
        let mut best_score: super::hand_evaluation::HandScore = 0;
        let mut winners = Vec::new();
        for &seat in &active_seats {
            let hole = &self.hole_cards[seat as usize];
            let cards: Vec<_> = hole.iter().chain(self.community_cards.iter()).copied().collect();
            let score = super::hand_evaluation::evaluate_hand_score(&cards);
            if score > best_score {
                best_score = score;
                winners.clear();
                winners.push(seat);
            } else if score == best_score {
                winners.push(seat);
            }
        }
        winners
    }

    pub fn validate(&self) -> Result<(), String> {
        if !seat_is_valid(self.button_position) {
            return Err(format!("invalid button position {}", self.button_position));
        }
        if self.small_blind >= self.big_blind {
            return Err(format!(
                "small_blind ({}) must be strictly less than big_blind ({})",
                self.small_blind, self.big_blind
            ));
        }
        // hole_cards must have exactly 2 cards per seat
        for (i, cards) in self.hole_cards.iter().enumerate() {
            if cards.len() != HOLE_CARDS_PER_SEAT {
                return Err(format!(
                    "hole_cards[{}] length {} != {}",
                    i,
                    cards.len(),
                    HOLE_CARDS_PER_SEAT
                ));
            }
        }
        // community_cards length must match street
        let expected_community = match self.current_street {
            Street::PreFlop => 0,
            Street::Flop => 3,
            Street::Turn => 4,
            Street::River | Street::Showdown | Street::Finished => 5,
        };
        if self.community_cards.len() != expected_community {
            return Err(format!(
                "community_cards length {} does not match street {:?} (expected {})",
                self.community_cards.len(),
                self.current_street,
                expected_community
            ));
        }
        // pot validation
        self.pot.validate()?;
        // validate each action
        for (i, action) in self.actions.iter().enumerate() {
            action.validate().map_err(|e| format!("action {i} invalid: {e}"))?;
        }
        Ok(())
    }
}
