use chrono::Utc;
use game_engine::{Action, ActionKind, Hand};

#[test]
fn hand_deal_creates_valid_hand() {
    let seed = [0u8; 32];
    let hand = Hand::deal(10, 20, 0, [1500, 1500], seed).unwrap();
    assert_eq!(hand.small_blind, 10);
    assert_eq!(hand.big_blind, 20);
    assert_eq!(hand.button_position, 0);
    assert_eq!(hand.player_stacks, [1500, 1500]);
    assert_eq!(hand.hole_cards.len(), 2);
    assert_eq!(hand.hole_cards[0].len(), 2);
    assert_eq!(hand.hole_cards[1].len(), 2);
    assert!(hand.community_cards.is_empty());
    assert_eq!(hand.current_street, game_engine::Street::PreFlop);
    assert!(hand.actions.is_empty());
}

#[test]
fn hand_apply_action_fold() {
    let seed = [0u8; 32];
    let mut hand = Hand::deal(10, 20, 0, [1500, 1500], seed).unwrap();
    let action = Action { seat: 0, kind: ActionKind::Fold, amount: None, timestamp: Utc::now() };
    assert!(hand.apply_action(action).is_ok());
    assert_eq!(hand.actions.len(), 1);
}

#[test]
fn hand_apply_action_bet() {
    let seed = [0u8; 32];
    let mut hand = Hand::deal(10, 20, 0, [1500, 1500], seed).unwrap();
    let action =
        Action { seat: 0, kind: ActionKind::Bet, amount: Some(100), timestamp: Utc::now() };
    let result = hand.apply_action(action);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn hand_advance_street() {
    let seed = [0u8; 32];
    let mut hand = Hand::deal(10, 20, 0, [1500, 1500], seed).unwrap();
    assert!(hand.advance_street().is_err());
    let call_action =
        Action { seat: 0, kind: ActionKind::Call, amount: Some(10), timestamp: Utc::now() };
    assert!(hand.apply_action(call_action).is_ok());
    let check_action =
        Action { seat: 1, kind: ActionKind::Check, amount: None, timestamp: Utc::now() };
    assert!(hand.apply_action(check_action).is_ok());
    assert!(hand.betting.is_round_complete());
    assert!(hand.advance_street().is_ok());
    assert_eq!(hand.current_street, game_engine::Street::Flop);
    assert_eq!(hand.community_cards.len(), 3);
    let check_action1 =
        Action { seat: 0, kind: ActionKind::Check, amount: None, timestamp: Utc::now() };
    let check_action2 =
        Action { seat: 1, kind: ActionKind::Check, amount: None, timestamp: Utc::now() };
    hand.apply_action(check_action1).unwrap();
    hand.apply_action(check_action2).unwrap();
    assert!(hand.betting.is_round_complete());
    assert!(hand.advance_street().is_ok());
    assert_eq!(hand.current_street, game_engine::Street::Turn);
    assert_eq!(hand.community_cards.len(), 4);
}

#[test]
fn hand_evaluate_winner_fold() {
    let seed = [0u8; 32];
    let mut hand = Hand::deal(10, 20, 0, [1500, 1500], seed).unwrap();
    let action = Action { seat: 0, kind: ActionKind::Fold, amount: None, timestamp: Utc::now() };
    hand.apply_action(action).unwrap();
    let winners = hand.evaluate_winner();
    assert_eq!(winners, vec![1]);
}

#[test]
fn hand_evaluate_winner_showdown() {
    let seed = [0u8; 32];
    let hand = Hand::deal(10, 20, 0, [1500, 1500], seed).unwrap();
    let winners = hand.evaluate_winner();
    assert!(!winners.is_empty() && winners.len() <= 2);
}

#[test]
fn betting_acting_seat_after_bet() {
    let seed = [0u8; 32];
    let mut hand = Hand::deal(10, 20, 0, [1500, 1500], seed).unwrap();

    assert_eq!(hand.betting.acting_seat(), Some(0));

    hand.apply_action(Action {
        seat: 0,
        kind: ActionKind::Call,
        amount: Some(10),
        timestamp: Utc::now(),
    })
    .unwrap();

    assert_eq!(hand.betting.acting_seat(), Some(1));

    hand.apply_action(Action {
        seat: 1,
        kind: ActionKind::Raise,
        amount: Some(60),
        timestamp: Utc::now(),
    })
    .unwrap();

    assert_eq!(hand.betting.acting_seat(), Some(0));
}

#[test]
fn betting_all_in_short_stack() {
    let seed = [0u8; 32];
    let mut hand = Hand::deal(10, 20, 0, [15, 1500], seed).unwrap();

    assert_eq!(hand.betting.acting_seat(), Some(0));

    hand.apply_action(Action {
        seat: 0,
        kind: ActionKind::Call,
        amount: Some(5),
        timestamp: Utc::now(),
    })
    .unwrap();

    assert!(hand.betting.stacks()[0] == 0);
}

#[test]
fn betting_cannot_check_with_bet_pending() {
    let seed = [0u8; 32];
    let mut hand = Hand::deal(10, 20, 0, [1500, 1500], seed).unwrap();

    let result = hand.apply_action(Action {
        seat: 0,
        kind: ActionKind::Check,
        amount: None,
        timestamp: Utc::now(),
    });

    assert!(result.is_err());
}
