use serde::{Deserialize, Serialize};

use super::{ChipCount, Seat};

/// Pot(s) in the current hand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pot {
    pub main: ChipCount,
    pub side_pots: Vec<SidePot>, // only if all‑in situations
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidePot {
    pub amount: ChipCount,
    pub eligible_seats: Vec<Seat>, // players who contributed to this side pot
}

impl Pot {
    pub fn total(&self) -> ChipCount {
        self.main + self.side_pots.iter().map(|p| p.amount).sum::<ChipCount>()
    }

    pub fn validate(&self) -> Result<(), String> {
        // main pot can be zero
        for (i, side) in self.side_pots.iter().enumerate() {
            if side.amount == 0 {
                return Err(format!("side pot {i} amount is zero"));
            }
            if side.eligible_seats.is_empty() {
                return Err(format!("side pot {i} has no eligible seats"));
            }
            for &seat in &side.eligible_seats {
                if !super::hand::seat_is_valid(seat) {
                    return Err(format!("side pot {i} invalid seat {seat}"));
                }
            }
            // ensure no duplicate seats? maybe fine
        }
        Ok(())
    }
}
