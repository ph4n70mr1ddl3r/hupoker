use serde::{Deserialize, Serialize};

use super::Seat;

pub type ChipCount = u64;

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
                return Err(format!("side pot {} amount is zero", i));
            }
            if side.eligible_seats.is_empty() {
                return Err(format!("side pot {} has no eligible seats", i));
            }
            for &seat in &side.eligible_seats {
                if !super::hand::seat_is_valid(seat) {
                    return Err(format!("side pot {} invalid seat {}", i, seat));
                }
            }
            // ensure no duplicate seats? maybe fine
        }
        Ok(())
    }
}