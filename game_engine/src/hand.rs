use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::{Action, ActionError, Card, ChipCount, Pot};

pub type Seat = u8;

const NUM_SEATS: u8 = 2;
const HOLE_CARDS_PER_SEAT: usize = 2;

pub fn seat_is_valid(seat: Seat) -> bool {
    seat < NUM_SEATS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandId(Uuid);

impl HandId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for HandId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for HandId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(s).map_err(|e| format!("invalid hand ID: {}", e))?;
        Ok(Self(uuid))
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

#[derive(Debug, Error)]
pub enum HandError {
    #[error("failed to post blinds: {0}")]
    BlindsFailed(#[from] super::betting::BettingError),
    #[error("invalid seat: {0}")]
    InvalidSeat(Seat),
    #[error("cannot advance street while betting round is incomplete")]
    BettingIncomplete,
    #[error("hand already finished")]
    AlreadyFinished,
    #[error("invalid button position: {0}")]
    InvalidButtonPosition(Seat),
    #[error("small blind ({0}) must be strictly less than big blind ({1})")]
    InvalidBlinds(ChipCount, ChipCount),
    #[error("hole_cards[{0}] length {1} != {2}")]
    InvalidHoleCards(usize, usize, usize),
    #[error("community_cards length {0} does not match street {1:?} (expected {2})")]
    InvalidCommunityCards(usize, Street, usize),
    #[error("pot validation failed: {0}")]
    PotValidation(String),
    #[error("action {0} invalid: {1}")]
    InvalidAction(usize, ActionError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hand {
    pub id: HandId,
    pub deck: super::Deck,
    #[serde(skip)]
    pub betting: super::betting::Betting,
    pub small_blind: ChipCount,
    pub big_blind: ChipCount,
    pub hole_cards: [Vec<Card>; 2],
    pub community_cards: Vec<Card>,
    pub pot: Pot,
    pub current_street: Street,
    pub actions: Vec<Action>,
    pub player_stacks: [ChipCount; 2],
    pub button_position: Seat,
    pub last_action_time: Option<DateTime<Utc>>,
}

impl Hand {
    pub fn deal(
        small_blind: ChipCount,
        big_blind: ChipCount,
        button_position: Seat,
        player_stacks: [ChipCount; 2],
        seed: [u8; 32],
    ) -> Result<Self, HandError> {
        let deck = super::Deck::new(seed);
        let betting =
            super::betting::Betting::new(small_blind, big_blind, player_stacks, button_position)?;
        let mut deck = deck;
        let mut hole_cards = [Vec::new(), Vec::new()];
        for _ in 0..HOLE_CARDS_PER_SEAT {
            for cards in &mut hole_cards {
                if let Some(card) = deck.draw() {
                    cards.push(card);
                }
            }
        }
        let pot = super::Pot { main: 0, side_pots: Vec::new() };
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

    pub fn advance_street(&mut self) -> Result<(), HandError> {
        if !self.betting.is_round_complete() {
            return Err(HandError::BettingIncomplete);
        }
        self.pot.main = self.pot.main.saturating_add(self.betting.total_pot());
        self.betting = super::betting::Betting::new_street(
            self.big_blind,
            self.betting.stacks(),
            self.button_position,
        );
        self.last_action_time = Some(Utc::now());
        match self.current_street {
            Street::PreFlop => {
                for _ in 0..3 {
                    if let Some(card) = self.deck.draw() {
                        self.community_cards.push(card);
                    }
                }
                self.current_street = Street::Flop;
            }
            Street::Flop => {
                if let Some(card) = self.deck.draw() {
                    self.community_cards.push(card);
                }
                self.current_street = Street::Turn;
            }
            Street::Turn => {
                if let Some(card) = self.deck.draw() {
                    self.community_cards.push(card);
                }
                self.current_street = Street::River;
            }
            Street::River => {
                self.current_street = Street::Showdown;
            }
            Street::Showdown => {
                self.current_street = Street::Finished;
            }
            Street::Finished => {
                return Err(HandError::AlreadyFinished);
            }
        }
        Ok(())
    }

    pub fn apply_action(&mut self, action: Action) -> Result<(), HandError> {
        if !seat_is_valid(action.seat) {
            return Err(HandError::InvalidSeat(action.seat));
        }
        self.betting
            .apply_action(action.seat, action.kind, action.amount)
            .map_err(HandError::BlindsFailed)?;
        self.player_stacks = self.betting.stacks();
        self.actions.push(action);
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

    pub fn validate(&self) -> Result<(), HandError> {
        if !seat_is_valid(self.button_position) {
            return Err(HandError::InvalidButtonPosition(self.button_position));
        }
        if self.small_blind >= self.big_blind {
            return Err(HandError::InvalidBlinds(self.small_blind, self.big_blind));
        }
        for (i, cards) in self.hole_cards.iter().enumerate() {
            if cards.len() != HOLE_CARDS_PER_SEAT {
                return Err(HandError::InvalidHoleCards(i, cards.len(), HOLE_CARDS_PER_SEAT));
            }
        }
        let expected_community = match self.current_street {
            Street::PreFlop => 0,
            Street::Flop => 3,
            Street::Turn => 4,
            Street::River | Street::Showdown | Street::Finished => 5,
        };
        if self.community_cards.len() != expected_community {
            return Err(HandError::InvalidCommunityCards(
                self.community_cards.len(),
                self.current_street,
                expected_community,
            ));
        }
        self.pot.validate().map_err(HandError::PotValidation)?;
        for (i, action) in self.actions.iter().enumerate() {
            action.validate().map_err(|e| HandError::InvalidAction(i, e))?;
        }
        Ok(())
    }
}
