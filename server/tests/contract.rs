use chrono::Utc;
use game_engine::{
    Action, ActionKind, Card, ConnectionId, HandId, Player, Pot, Rank, Street, Suit, TableConfig,
    TableId,
};

use server::protocol::messages::*;
use uuid::Uuid;

#[test]
fn client_hello_serialization() {
    let msg = Message::ClientHello {
        version: "1.0".to_string(),
        client_name: "test-client".to_string(),
        client_version: "1.0".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    // Ensure it can be deserialized
    let decoded: Message = serde_json::from_str(&json).unwrap();
    match decoded {
        Message::ClientHello { version, client_name, client_version } => {
            assert_eq!(version, "1.0");
            assert_eq!(client_name, "test-client");
            assert_eq!(client_version, "1.0");
        }
        _ => panic!("wrong message type"),
    }
}

#[test]
fn join_table_serialization() {
    let msg = Message::JoinTable {
        version: "1.0".to_string(),
        table_id: TableId::new("table-1".to_string()),
        seat: 0,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: Message = serde_json::from_str(&json).unwrap();
    match decoded {
        Message::JoinTable { version, table_id, seat } => {
            assert_eq!(version, "1.0");
            assert_eq!(table_id.as_str(), "table-1");
            assert_eq!(seat, 0);
        }
        _ => panic!("wrong message type"),
    }
}

#[test]
fn action_message_serialization() {
    let hand_id = HandId::new(Uuid::new_v4());
    let msg = Message::Action {
        version: "1.0".to_string(),
        hand_id,
        kind: ActionKind::Raise,
        amount: Some(100),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: Message = serde_json::from_str(&json).unwrap();
    match decoded {
        Message::Action { version, hand_id: decoded_hand_id, kind, amount } => {
            assert_eq!(version, "1.0");
            assert_eq!(decoded_hand_id, hand_id);
            assert_eq!(kind, ActionKind::Raise);
            assert_eq!(amount, Some(100));
        }
        _ => panic!("wrong message type"),
    }
}

#[test]
fn server_hello_serialization() {
    let msg = Message::ServerHello {
        version: "1.0".to_string(),
        status: "accepted".to_string(),
        server_name: "test-server".to_string(),
        server_version: "1.0".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: Message = serde_json::from_str(&json).unwrap();
    match decoded {
        Message::ServerHello { version, status, server_name, server_version } => {
            assert_eq!(version, "1.0");
            assert_eq!(status, "accepted");
            assert_eq!(server_name, "test-server");
            assert_eq!(server_version, "1.0");
        }
        _ => panic!("wrong message type"),
    }
}

#[test]
fn table_state_serialization() {
    let player = Player {
        seat: 0,
        stack: 1500,
        connection_id: ConnectionId::new(Uuid::new_v4()),
        disconnected_at: None,
        is_sitting_out: false,
    };
    let seat = TableSeat { seat: 0, player: Some(player) };
    let config = TableConfig {
        small_blind: 10,
        big_blind: 20,
        starting_stack: 1500,
        action_timeout_secs: 30,
        reconnection_timeout_secs: 60,
    };
    let msg = Message::TableState {
        version: "1.0".to_string(),
        table_id: TableId::new("table-1".to_string()),
        seats: vec![seat],
        config,
        current_hand_id: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: Message = serde_json::from_str(&json).unwrap();
    match decoded {
        Message::TableState { version, table_id, seats, config, current_hand_id } => {
            assert_eq!(version, "1.0");
            assert_eq!(table_id.as_str(), "table-1");
            assert_eq!(seats.len(), 1);
            assert_eq!(seats[0].seat, 0);
            assert!(seats[0].player.is_some());
            assert_eq!(config.small_blind, 10);
            assert_eq!(current_hand_id, None);
        }
        _ => panic!("wrong message type"),
    }
}

#[test]
fn hand_state_serialization() {
    let hand_id = HandId::new(Uuid::new_v4());
    let table_id = TableId::new("table-1".to_string());
    let hole_cards = vec![
        Card { rank: Rank::Ace, suit: Suit::Spades },
        Card { rank: Rank::King, suit: Suit::Diamonds },
    ];
    let community_cards = vec![
        Card { rank: Rank::Two, suit: Suit::Spades },
        Card { rank: Rank::Three, suit: Suit::Spades },
        Card { rank: Rank::Four, suit: Suit::Spades },
    ];
    let pot = Pot { main: 320, side_pots: vec![] };
    let actions = vec![Action {
        seat: 0,
        kind: game_engine::ActionKind::Bet,
        amount: Some(100),
        timestamp: Utc::now(),
    }];
    let msg = Message::HandState {
        version: "1.0".to_string(),
        hand_id,
        table_id,
        hole_cards,
        community_cards,
        pot,
        current_street: Street::Flop,
        actions,
        player_stacks: [1400, 1500],
        button_position: 0,
        last_action_time: Utc::now().to_rfc3339(),
        acting_seat: Some(1),
        time_remaining_ms: 25000,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: Message = serde_json::from_str(&json).unwrap();
    match decoded {
        Message::HandState {
            version,
            hand_id: decoded_hand_id,
            table_id: decoded_table_id,
            hole_cards: decoded_hole_cards,
            community_cards: decoded_community_cards,
            pot: decoded_pot,
            current_street,
            actions: _decoded_actions,
            player_stacks,
            button_position,
            last_action_time: _,
            acting_seat,
            time_remaining_ms: _,
        } => {
            assert_eq!(version, "1.0");
            assert_eq!(decoded_hand_id, hand_id);
            assert_eq!(decoded_table_id.as_str(), "table-1");
            assert_eq!(decoded_hole_cards.len(), 2);
            assert_eq!(decoded_community_cards.len(), 3);
            assert_eq!(decoded_pot.main, 320);
            assert_eq!(current_street, Street::Flop);
            assert_eq!(player_stacks[0], 1400);
            assert_eq!(button_position, 0);
            assert_eq!(acting_seat, Some(1));
        }
        _ => panic!("wrong message type"),
    }
}
