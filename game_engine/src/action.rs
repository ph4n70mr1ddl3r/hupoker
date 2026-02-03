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

/// Possible kinds of poker actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    Fold,
    Check,
    Call,
    Bet,
    Raise,
}

impl std::fmt::Display for ActionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionKind::Fold => write!(f, "Fold"),
            ActionKind::Check => write!(f, "Check"),
            ActionKind::Call => write!(f, "Call"),
            ActionKind::Bet => write!(f, "Bet"),
            ActionKind::Raise => write!(f, "Raise"),
        }
    }
}

impl Action {
    /// Validates that the action is semantically correct.
    ///
    /// Returns `Ok(())` if valid, otherwise an error string.
    pub fn validate(&self) -> Result<(), String> {
        if !super::hand::seat_is_valid(self.seat) {
            return Err(format!("invalid seat {}", self.seat));
        }
        match self.kind {
            ActionKind::Fold | ActionKind::Check => {
                if self.amount.is_some() {
                    return Err(format!("{:?} must not have amount", self.kind));
                }
            }
            ActionKind::Call | ActionKind::Bet | ActionKind::Raise => {
                if self.amount.is_none() {
                    return Err(format!("{:?} must have amount", self.kind));
                }
                let amount = self.amount.unwrap();
                if amount == 0 {
                    return Err(format!("{:?} amount must be positive", self.kind));
                }
            }
        }
        Ok(())
    }
}
