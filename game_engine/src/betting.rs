use super::{ActionKind, ChipCount, Seat};

#[derive(Debug, Clone, Default)]
pub struct Betting {
    // bets placed by each player in the current betting round (not total contributions)
    bets: [ChipCount; 2],
    // current highest bet in this round (including previous raises)
    current_high: ChipCount,
    // minimum raise amount (big blind initially, then difference between raises)
    min_raise: ChipCount,
    // total chips in the pot (sum of all bets from all streets)
    total_pot: ChipCount,
    // whether the betting round is complete (both players acted and bets are equal, or all-in situation)
    round_complete: bool,
    // player stacks (remaining chips) - needed to validate all-in
    stacks: [ChipCount; 2],
    // which seats have acted in the current betting round
    acted_this_round: [bool; 2],
    // which seats are all-in
    all_in: [bool; 2],
}

#[derive(Debug, thiserror::Error)]
pub enum BettingError {
    #[error("invalid bet amount: amount must be provided for bet/raise/call actions")]
    InvalidAmount,
    #[error("insufficient stack: player has {0} chips but needs {1}")]
    InsufficientStack(ChipCount, ChipCount),
    #[error("bet below minimum raise: total bet must be at least {0}, got {1}")]
    BelowMinRaise(ChipCount, ChipCount),
    #[error("action out of turn: it is seat {0}'s turn to act")]
    OutOfTurn(Seat),
    #[error("illegal action: {0}")]
    IllegalAction(String),
    #[error("betting round already complete")]
    RoundComplete,
}

impl Betting {
    /// Starts a new betting round (e.g., preflop, flop, turn, river).
    /// `small_blind` and `big_blind` are used for minimum bet sizes (preflop only).
    /// `stacks` are current player stacks before posting blinds.
    /// `button_position` determines who posts small blind (seat 0) and big blind (seat 1).
    pub fn new(
        small_blind: ChipCount,
        big_blind: ChipCount,
        stacks: [ChipCount; 2],
        button_position: Seat,
    ) -> Self {
        let mut bets = [0, 0];
        // post blinds
        let small_blind_seat = button_position;
        let big_blind_seat = 1 - button_position;
        bets[small_blind_seat as usize] = small_blind;
        bets[big_blind_seat as usize] = big_blind;
        // after posting blinds, current high is big blind, min raise is big blind (difference)
        let current_high = big_blind;
        let min_raise = big_blind; // minimum raise is the big blind amount (difference)
        let total_pot = small_blind + big_blind;
        let mut new_stacks = stacks;
        new_stacks[small_blind_seat as usize] -= small_blind;
        new_stacks[big_blind_seat as usize] -= big_blind;
        Self {
            bets,
            current_high,
            min_raise,
            total_pot,
            round_complete: false,
            stacks: new_stacks,
            acted_this_round: [false, false],
            all_in: [false, false],
        }
    }

    /// Starts a new betting round for post‑flop streets (no blinds posted).
    /// `big_blind` is used for minimum bet size.
    /// `stacks` are current player stacks.
    pub fn new_street(big_blind: ChipCount, stacks: [ChipCount; 2]) -> Self {
        Self {
            bets: [0, 0],
            current_high: 0,
            min_raise: big_blind,
            total_pot: 0,
            round_complete: false,
            stacks,
            acted_this_round: [false, false],
            all_in: [false, false],
        }
    }

    /// Returns the amount a player must call to stay in the hand.
    pub fn amount_to_call(&self, seat: Seat) -> ChipCount {
        self.current_high - self.bets[seat as usize]
    }

    /// Returns true if the betting round is complete (both players have acted and bets are equal).
    pub fn is_round_complete(&self) -> bool {
        self.round_complete
    }

    /// Returns the current bets for each seat.
    pub fn bets(&self) -> [ChipCount; 2] {
        self.bets
    }

    /// Returns the total pot size.
    pub fn total_pot(&self) -> ChipCount {
        self.total_pot
    }

    /// Returns player stacks after accounting for bets this round.
    pub fn stacks(&self) -> [ChipCount; 2] {
        self.stacks
    }

    /// Applies a player action.
    /// `seat` must be the player whose turn it is.
    /// `kind` and `amount` describe the action.
    /// Returns updated betting state or an error.
    pub fn apply_action(
        &mut self,
        seat: Seat,
        kind: ActionKind,
        amount: Option<ChipCount>,
    ) -> Result<(), BettingError> {
        if self.round_complete {
            return Err(BettingError::RoundComplete);
        }
        match kind {
            ActionKind::Fold => {
                // Fold ends the hand immediately; betting round complete.
                self.round_complete = true;
                // No further actions needed.
                Ok(())
            }
            ActionKind::Check => {
                // Check only allowed if amount_to_call == 0
                let amount_to_call = self.amount_to_call(seat);
                if amount_to_call != 0 {
                    return Err(BettingError::IllegalAction(format!(
                        "cannot check: must call {} chips",
                        amount_to_call
                    )));
                }
                // Mark that player has acted; need to check if both players acted and bets equal.
                self.mark_action(seat);
                Ok(())
            }
            ActionKind::Call => {
                let call_amount = self.amount_to_call(seat);
                if call_amount == 0 {
                    // Essentially a check
                    self.mark_action(seat);
                    return Ok(());
                }
                // Validate amount matches call_amount (or all-in)
                let amount = amount.ok_or(BettingError::InvalidAmount)?;
                if amount > self.stacks[seat as usize] {
                    return Err(BettingError::InsufficientStack(
                        self.stacks[seat as usize],
                        amount,
                    ));
                }
                // Player can call less if all-in (amount < call_amount)
                let actual_call = amount.min(call_amount);
                self.bets[seat as usize] = self.bets[seat as usize].saturating_add(actual_call);
                self.total_pot = self.total_pot.saturating_add(actual_call);
                self.stacks[seat as usize] = self.stacks[seat as usize].saturating_sub(actual_call);
                // Mark as all-in if stack is now zero
                if self.stacks[seat as usize] == 0 {
                    self.all_in[seat as usize] = true;
                }
                // If player went all-in and amount < call_amount, they can't do more
                // The betting round may still continue if the other player can raise
                self.mark_action(seat);
                Ok(())
            }
            ActionKind::Bet => {
                // Bet only allowed if current_high == 0 (no previous bet this round)
                if self.current_high != 0 {
                    return Err(BettingError::IllegalAction(
                        "cannot bet: there is already a bet in this round, use raise instead"
                            .to_string(),
                    ));
                }
                let bet_amount = amount.ok_or(BettingError::InvalidAmount)?;
                if bet_amount > self.stacks[seat as usize] {
                    return Err(BettingError::InsufficientStack(
                        self.stacks[seat as usize],
                        bet_amount,
                    ));
                }
                // Bet must be at least the big blind (min_raise)
                if bet_amount < self.min_raise {
                    return Err(BettingError::BelowMinRaise(self.min_raise, bet_amount));
                }
                self.bets[seat as usize] = self.bets[seat as usize].saturating_add(bet_amount);
                self.current_high = bet_amount;
                self.min_raise = bet_amount; // minimum raise becomes the bet amount (difference)
                self.total_pot = self.total_pot.saturating_add(bet_amount);
                self.stacks[seat as usize] = self.stacks[seat as usize].saturating_sub(bet_amount);
                // Mark as all-in if stack is now zero
                if self.stacks[seat as usize] == 0 {
                    self.all_in[seat as usize] = true;
                }
                // Reset acted flags because a new bet level resets the round
                self.acted_this_round = [false, false];
                self.mark_action(seat);
                Ok(())
            }
            ActionKind::Raise => {
                // Raise allowed only if there is a previous bet (current_high > 0)
                if self.current_high == 0 {
                    return Err(BettingError::IllegalAction(
                        "cannot raise: no bet to raise, use bet instead".to_string(),
                    ));
                }
                let raise_amount = amount.ok_or(BettingError::InvalidAmount)?;
                if raise_amount > self.stacks[seat as usize] {
                    return Err(BettingError::InsufficientStack(
                        self.stacks[seat as usize],
                        raise_amount,
                    ));
                }
                // Total bet after raise must be at least current_high + min_raise
                let total_bet_after = self.bets[seat as usize].saturating_add(raise_amount);
                let min_total_bet = self.current_high.saturating_add(self.min_raise);
                if total_bet_after < min_total_bet {
                    // Unless player is all-in (raise_amount equals their remaining stack)
                    let is_all_in = raise_amount == self.stacks[seat as usize];
                    if !is_all_in {
                        return Err(BettingError::BelowMinRaise(min_total_bet, total_bet_after));
                    }
                    // All-in raise less than min raise is allowed
                }
                // Update
                let additional = raise_amount;
                self.bets[seat as usize] = self.bets[seat as usize].saturating_add(additional);
                self.total_pot = self.total_pot.saturating_add(additional);
                self.stacks[seat as usize] = self.stacks[seat as usize].saturating_sub(additional);
                // Mark as all-in if stack is now zero
                if self.stacks[seat as usize] == 0 {
                    self.all_in[seat as usize] = true;
                }
                if total_bet_after > self.current_high {
                    self.min_raise = total_bet_after.saturating_sub(self.current_high);
                    self.current_high = total_bet_after;
                    // Reset acted flags because a raise resets the round
                    self.acted_this_round = [false, false];
                }
                self.mark_action(seat);
                Ok(())
            }
        }
    }

    /// Mark that a player has acted this round. If both players have acted and bets are equal,
    /// or if one player is all-in and the other has acted, the betting round is complete.
    fn mark_action(&mut self, seat: Seat) {
        // Mark this seat as acted
        self.acted_this_round[seat as usize] = true;
        // Check for betting round completion:
        // 1. Both players have acted and bets are equal, OR
        // 2. Both players are all-in, OR
        // 3. One player is all-in and both have acted (even if bets not equal)
        let both_acted = self.acted_this_round[0] && self.acted_this_round[1];
        let both_all_in = self.all_in[0] && self.all_in[1];
        let one_all_in = self.all_in[0] || self.all_in[1];
        let bets_equal = self.bets[0] == self.bets[1];

        if both_acted && (bets_equal || both_all_in || one_all_in) {
            self.round_complete = true;
        }
    }

    /// Returns the seat that should act next, or None if the betting round is complete.
    pub fn acting_seat(&self, button_position: Seat) -> Option<Seat> {
        if self.round_complete {
            return None;
        }
        // Determine which seats have acted this round
        if !self.acted_this_round[0] && !self.acted_this_round[1] {
            // No one has acted yet; first to act is button_position
            Some(button_position)
        } else if self.acted_this_round[0] && !self.acted_this_round[1] {
            // Seat 0 acted, seat 1 hasn't
            Some(1)
        } else if !self.acted_this_round[0] && self.acted_this_round[1] {
            // Seat 1 acted, seat 0 hasn't
            Some(0)
        } else {
            // Both have acted (but round not complete?) should not happen
            None
        }
    }
}
