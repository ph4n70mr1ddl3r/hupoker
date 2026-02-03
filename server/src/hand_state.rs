use chrono::Utc;
use game_engine::{Hand, Seat};

use crate::protocol::messages::{HandState, Message};

const PROTOCOL_VERSION: &str = "1.0";

/// Creates a HandState message for a specific player.
/// `hand` is the current hand.
/// `table_id` identifies the table.
/// `player_seat` is the seat of the player receiving this message (their hole cards are included).
/// `acting_seat` is the seat whose turn it is (if any).
/// `time_remaining_ms` is the remaining time for the acting player.
pub fn create_hand_state(
    hand: &Hand,
    table_id: game_engine::TableId,
    player_seat: Seat,
    acting_seat: Option<Seat>,
    time_remaining_ms: u64,
) -> HandState {
    let hole_cards =
        if player_seat < 2 { hand.hole_cards[player_seat as usize].clone() } else { Vec::new() };
    let last_action_time =
        hand.last_action_time.map(|dt| dt.to_rfc3339()).unwrap_or_else(|| Utc::now().to_rfc3339());
    HandState {
        hand_id: hand.id,
        table_id,
        hole_cards,
        community_cards: hand.community_cards.clone(),
        pot: hand.pot.clone(),
        current_street: hand.current_street,
        actions: hand.actions.clone(),
        player_stacks: hand.player_stacks,
        button_position: hand.button_position,
        last_action_time,
        acting_seat,
        time_remaining_ms,
    }
}

/// Creates a HandState Message (wrapper) for a player.
pub fn create_hand_state_message(
    hand: &Hand,
    table_id: game_engine::TableId,
    player_seat: Seat,
    acting_seat: Option<Seat>,
    time_remaining_ms: u64,
) -> Message {
    let hand_state = create_hand_state(hand, table_id, player_seat, acting_seat, time_remaining_ms);
    Message::HandState {
        version: PROTOCOL_VERSION.to_string(),
        hand_id: hand_state.hand_id,
        table_id: hand_state.table_id,
        hole_cards: hand_state.hole_cards,
        community_cards: hand_state.community_cards,
        pot: hand_state.pot,
        current_street: hand_state.current_street,
        actions: hand_state.actions,
        player_stacks: hand_state.player_stacks,
        button_position: hand_state.button_position,
        last_action_time: hand_state.last_action_time,
        acting_seat: hand_state.acting_seat,
        time_remaining_ms: hand_state.time_remaining_ms,
    }
}
