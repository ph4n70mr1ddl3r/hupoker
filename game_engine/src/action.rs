use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Seat;

/// A player’s decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub seat: Seat,
    pub kind: ActionKind,
    pub amount: Option<u64>, // Some for bet/raise/call, None for fold/check
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    Fold,
    Check,
    Call,
    Bet,
    Raise,
}

impl Action {
    pub fn validate(&self) -> Result<(), String> {
        if !super::hand::seat_is_valid(self.seat) {
            return Err(format!("invalid seat {}", self.seat));
        }
        match self.kind {
            ActionKind::Fold | ActionKind::Check => {
                if self.amount.is_some() {
                    return Err(format!("{} must not have amount", self.kind));
                }
            }
            ActionKind::Call | ActionKind::Bet | ActionKind::Raise => {
                if self.amount.is_none() {
                    return Err(format!("{} must have amount", self.kind));
                }
                let amount = self.amount.unwrap();
                if amount == 0 {
                    return Err(format!("{} amount must be positive", self.kind));
                }
                // TODO: validate amount against minimum raise, stack etc.
            }
        }
        Ok(())
    }
}