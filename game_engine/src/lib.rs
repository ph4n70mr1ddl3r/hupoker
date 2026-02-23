//! Game engine for heads-up NLHE poker.

mod action;
mod betting;
mod card;
mod config;
mod deck;
mod hand;
mod hand_evaluation;
mod player;
mod pot;
mod table;

pub type ChipCount = u64;

pub use action::{Action, ActionKind};
pub use betting::{Betting, BettingError};
pub use card::{Card, Rank, Suit};
pub use config::ServerConfig;
pub use deck::Deck;
pub use hand::{seat_is_valid, Hand, HandId, Seat, Street};
pub use hand_evaluation::{evaluate_hand, evaluate_hand_score, score_to_rank, HandRank, HandScore};
pub use player::{ConnectionId, Player};
pub use pot::{Pot, SidePot};
pub use table::{Table, TableConfig, TableId};
