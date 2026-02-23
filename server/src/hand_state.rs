use game_engine::{Hand, Seat};

use crate::protocol::messages::Message;

const PROTOCOL_VERSION: &str = "1.0";

pub fn create_hand_state_message(
    hand: &Hand,
    table_id: game_engine::TableId,
    player_seat: Seat,
    acting_seat: Option<Seat>,
    time_remaining_ms: u64,
) -> Message {
    let hole_cards =
        if player_seat < 2 { hand.hole_cards[player_seat as usize].clone() } else { Vec::new() };
    let last_action_time = hand
        .last_action_time
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    Message::HandState {
        version: PROTOCOL_VERSION.to_string(),
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
