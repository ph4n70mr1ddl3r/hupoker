use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::Seat;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub seat: Seat,
    pub kind: ActionKind,
    pub amount: Option<u64>,
    pub timestamp: DateTime<Utc>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    Fold,
    Check,
    Call,
    Bet,
    Raise,
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ActionError {
    #[error("invalid seat: {0}")]
    InvalidSeat(Seat),
    #[error("{0:?} must not have amount")]
    UnexpectedAmount(ActionKind),
    #[error("{0:?} must have amount")]
    MissingAmount(ActionKind),
    #[error("{0:?} amount must be positive")]
    ZeroAmount(ActionKind),
}

impl std::fmt::Display for ActionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fold => write!(f, "Fold"),
            Self::Check => write!(f, "Check"),
            Self::Call => write!(f, "Call"),
            Self::Bet => write!(f, "Bet"),
            Self::Raise => write!(f, "Raise"),
        }
    }
}

impl std::str::FromStr for ActionKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Fold" => Ok(Self::Fold),
            "Check" => Ok(Self::Check),
            "Call" => Ok(Self::Call),
            "Bet" => Ok(Self::Bet),
            "Raise" => Ok(Self::Raise),
            _ => Err(format!("invalid action kind: {}", s)),
        }
    }
}

impl Action {
    pub fn validate(&self) -> Result<(), ActionError> {
        if !super::hand::seat_is_valid(self.seat) {
            return Err(ActionError::InvalidSeat(self.seat));
        }
        match self.kind {
            ActionKind::Fold | ActionKind::Check => {
                if self.amount.is_some() {
                    return Err(ActionError::UnexpectedAmount(self.kind));
                }
            }
            ActionKind::Call | ActionKind::Bet | ActionKind::Raise => {
                let amount = self.amount.ok_or(ActionError::MissingAmount(self.kind))?;
                if amount == 0 {
                    return Err(ActionError::ZeroAmount(self.kind));
                }
            }
        }
        Ok(())
    }
}
