use super::{ChipCount, Seat};

#[derive(Debug, Clone)]
pub struct Betting {
    // TODO: implement betting state
}

#[derive(Debug, thiserror::Error)]
pub enum BettingError {
    #[error("invalid bet amount")]
    InvalidAmount,
    #[error("insufficient stack")]
    InsufficientStack,
    #[error("bet below minimum raise")]
    BelowMinRaise,
    #[error("action out of turn")]
    OutOfTurn,
}

impl Betting {
    pub fn new() -> Self {
        todo!()
    }

    pub fn apply_action(&mut self, seat: Seat, kind: super::ActionKind, amount: Option<ChipCount>) -> Result<(), BettingError> {
        todo!()
    }
}