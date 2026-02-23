use chrono::Utc;
use game_engine::{Action, ActionKind, Hand, Street};

#[test]
fn full_hand_simulation_checkdown() {
    let seed = [0u8; 32];
    let mut hand = Hand::deal(10, 20, 0, [1500, 1500], seed).unwrap();

    println!(
        "Before call: bets {:?}, round_complete {}",
        hand.betting.bets(),
        hand.betting.is_round_complete()
    );
    hand.apply_action(Action {
        seat: 0,
        kind: ActionKind::Call,
        amount: Some(10),
        timestamp: Utc::now(),
    })
    .expect("call should succeed");
    println!(
        "After call: bets {:?}, round_complete {}",
        hand.betting.bets(),
        hand.betting.is_round_complete()
    );
    println!("Amount to call seat 1: {}", hand.betting.amount_to_call(1));
    hand.apply_action(Action {
        seat: 1,
        kind: ActionKind::Check,
        amount: None,
        timestamp: Utc::now(),
    })
    .expect("check should succeed");
    assert!(hand.betting.is_round_complete());
    hand.advance_street().expect("advance to flop should succeed");
    assert_eq!(hand.current_street, Street::Flop);
    assert_eq!(hand.community_cards.len(), 3);

    hand.apply_action(Action {
        seat: 0,
        kind: ActionKind::Check,
        amount: None,
        timestamp: Utc::now(),
    })
    .expect("check should succeed");
    hand.apply_action(Action {
        seat: 1,
        kind: ActionKind::Check,
        amount: None,
        timestamp: Utc::now(),
    })
    .expect("check should succeed");
    assert!(hand.betting.is_round_complete());
    hand.advance_street().expect("advance to turn should succeed");
    assert_eq!(hand.current_street, Street::Turn);
    assert_eq!(hand.community_cards.len(), 4);

    hand.apply_action(Action {
        seat: 0,
        kind: ActionKind::Check,
        amount: None,
        timestamp: Utc::now(),
    })
    .expect("check should succeed");
    hand.apply_action(Action {
        seat: 1,
        kind: ActionKind::Check,
        amount: None,
        timestamp: Utc::now(),
    })
    .expect("check should succeed");
    assert!(hand.betting.is_round_complete());
    hand.advance_street().expect("advance to river should succeed");
    assert_eq!(hand.current_street, Street::River);
    assert_eq!(hand.community_cards.len(), 5);

    hand.apply_action(Action {
        seat: 0,
        kind: ActionKind::Check,
        amount: None,
        timestamp: Utc::now(),
    })
    .expect("check should succeed");
    hand.apply_action(Action {
        seat: 1,
        kind: ActionKind::Check,
        amount: None,
        timestamp: Utc::now(),
    })
    .expect("check should succeed");
    assert!(hand.betting.is_round_complete());
    hand.advance_street().expect("advance to showdown should succeed");
    assert_eq!(hand.current_street, Street::Showdown);

    let winners = hand.evaluate_winner();
    assert!(!winners.is_empty());
    assert!(winners.len() <= 2);
    println!("Winners: {:?}", winners);
}

#[test]
fn full_hand_simulation_fold_preflop() {
    let seed = [0u8; 32];
    let mut hand = Hand::deal(10, 20, 0, [1500, 1500], seed).unwrap();

    hand.apply_action(Action {
        seat: 0,
        kind: ActionKind::Fold,
        amount: None,
        timestamp: Utc::now(),
    })
    .expect("fold should succeed");
    assert!(hand.betting.is_round_complete());
    let winners = hand.evaluate_winner();
    assert_eq!(winners, vec![1]);
}

#[test]
fn full_hand_simulation_bet_call() {
    let seed = [0u8; 32];
    let mut hand = Hand::deal(10, 20, 0, [1500, 1500], seed).unwrap();

    hand.apply_action(Action {
        seat: 0,
        kind: ActionKind::Raise,
        amount: Some(90),
        timestamp: Utc::now(),
    })
    .expect("raise should succeed");
    hand.apply_action(Action {
        seat: 1,
        kind: ActionKind::Call,
        amount: Some(80),
        timestamp: Utc::now(),
    })
    .expect("call should succeed");
    assert!(hand.betting.is_round_complete());
    hand.advance_street().expect("advance to flop");
    assert_eq!(hand.current_street, Street::Flop);
    hand.apply_action(Action {
        seat: 0,
        kind: ActionKind::Check,
        amount: None,
        timestamp: Utc::now(),
    })
    .expect("check");
    hand.apply_action(Action {
        seat: 1,
        kind: ActionKind::Check,
        amount: None,
        timestamp: Utc::now(),
    })
    .expect("check");
    hand.advance_street().expect("advance to turn");
}
