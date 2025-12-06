use super::Card;
use super::Rank;
use super::Suit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandRank {
    HighCard,
    OnePair,
    TwoPair,
    ThreeOfAKind,
    Straight,
    Flush,
    FullHouse,
    FourOfAKind,
    StraightFlush,
    RoyalFlush,
}

fn rank_value(rank: Rank) -> u8 {
    match rank {
        Rank::Two => 2,
        Rank::Three => 3,
        Rank::Four => 4,
        Rank::Five => 5,
        Rank::Six => 6,
        Rank::Seven => 7,
        Rank::Eight => 8,
        Rank::Nine => 9,
        Rank::Ten => 10,
        Rank::Jack => 11,
        Rank::Queen => 12,
        Rank::King => 13,
        Rank::Ace => 14,
    }
}

fn is_flush(cards: &[Card]) -> bool {
    if cards.len() < 5 {
        return false;
    }
    let mut suit_counts = [0u8; 4];
    for card in cards {
        match card.suit {
            Suit::Clubs => suit_counts[0] += 1,
            Suit::Diamonds => suit_counts[1] += 1,
            Suit::Hearts => suit_counts[2] += 1,
            Suit::Spades => suit_counts[3] += 1,
        }
    }
    suit_counts.iter().any(|&c| c >= 5)
}

fn is_straight(cards: &[Card]) -> bool {
    if cards.len() < 5 {
        return false;
    }
    let mut values: Vec<u8> = cards.iter().map(|c| rank_value(c.rank)).collect();
    values.sort();
    values.dedup();
    // handle ace low straight
    if values.contains(&14) {
        let mut low_values = values.clone();
        low_values.iter_mut().for_each(|v| {
            if *v == 14 {
                *v = 1
            }
        });
        low_values.sort();
        low_values.dedup();
        for window in low_values.windows(5) {
            if window[4] - window[0] == 4 {
                return true;
            }
        }
    }
    for window in values.windows(5) {
        if window[4] - window[0] == 4 {
            return true;
        }
    }
    false
}

fn count_ranks(cards: &[Card]) -> Vec<(Rank, u8)> {
    use std::collections::HashMap;
    let mut map = HashMap::new();
    for card in cards {
        *map.entry(card.rank).or_insert(0) += 1;
    }
    let mut counts: Vec<_> = map.into_iter().collect();
    counts.sort_by_key(|&(rank, count)| (count, rank_value(rank)));
    counts.reverse();
    counts
}

fn evaluate_5card_hand(cards: &[Card]) -> HandRank {
    assert!(cards.len() == 5);
    let counts = count_ranks(cards);
    let is_flush = is_flush(cards);
    let is_straight = is_straight(cards);

    if is_flush && is_straight {
        // check royal flush
        let mut values: Vec<u8> = cards.iter().map(|c| rank_value(c.rank)).collect();
        values.sort();
        if values.contains(&10)
            && values.contains(&11)
            && values.contains(&12)
            && values.contains(&13)
            && values.contains(&14)
        {
            return HandRank::RoyalFlush;
        }
        return HandRank::StraightFlush;
    }
    match counts[0].1 {
        4 => HandRank::FourOfAKind,
        3 => {
            if counts.len() >= 2 && counts[1].1 >= 2 {
                HandRank::FullHouse
            } else {
                HandRank::ThreeOfAKind
            }
        }
        2 => {
            if counts.len() >= 2 && counts[1].1 == 2 {
                HandRank::TwoPair
            } else {
                HandRank::OnePair
            }
        }
        _ => {
            if is_flush {
                HandRank::Flush
            } else if is_straight {
                HandRank::Straight
            } else {
                HandRank::HighCard
            }
        }
    }
}

pub fn evaluate_hand(cards: &[Card]) -> HandRank {
    if cards.len() < 5 {
        return HandRank::HighCard;
    }
    // Generate all combinations of 5 cards from the input
    let n = cards.len();
    let mut best = HandRank::HighCard;
    // brute force over all 5-card combinations (max 21 combos)
    for i in 0..n {
        for j in i + 1..n {
            for k in j + 1..n {
                for l in k + 1..n {
                    for m in l + 1..n {
                        let combo = [cards[i], cards[j], cards[k], cards[l], cards[m]];
                        let rank = evaluate_5card_hand(&combo);
                        if rank > best {
                            best = rank;
                        }
                    }
                }
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::super::Card;
    use super::super::Rank::*;
    use super::super::Suit::*;
    use super::*;

    fn card(rank: Rank, suit: Suit) -> Card {
        Card { rank, suit }
    }

    #[test]
    fn test_high_card() {
        let cards = vec![
            card(Two, Clubs),
            card(Four, Diamonds),
            card(Six, Hearts),
            card(Eight, Spades),
            card(Ten, Clubs),
            card(King, Diamonds),
            card(Ace, Hearts),
        ];
        assert_eq!(evaluate_hand(&cards), HandRank::HighCard);
    }

    #[test]
    fn test_one_pair() {
        let cards = vec![
            card(Two, Clubs),
            card(Two, Diamonds),
            card(Seven, Hearts),
            card(Eight, Spades),
            card(Ten, Clubs),
            card(Jack, Diamonds),
            card(King, Hearts),
        ];
        assert_eq!(evaluate_hand(&cards), HandRank::OnePair);
    }

    #[test]
    fn test_two_pair() {
        let cards = vec![
            card(Two, Clubs),
            card(Two, Diamonds),
            card(Three, Hearts),
            card(Three, Spades),
            card(Seven, Clubs),
            card(Eight, Diamonds),
            card(Ten, Hearts),
        ];
        assert_eq!(evaluate_hand(&cards), HandRank::TwoPair);
    }

    #[test]
    fn test_three_of_a_kind() {
        let cards = vec![
            card(Two, Clubs),
            card(Two, Diamonds),
            card(Two, Hearts),
            card(Seven, Spades),
            card(Eight, Clubs),
            card(Ten, Diamonds),
            card(Jack, Hearts),
        ];
        assert_eq!(evaluate_hand(&cards), HandRank::ThreeOfAKind);
    }

    #[test]
    fn test_straight() {
        let cards = vec![
            card(Two, Clubs),
            card(Three, Diamonds),
            card(Four, Hearts),
            card(Five, Spades),
            card(Six, Clubs),
            card(King, Diamonds),
            card(Ace, Hearts),
        ];
        assert_eq!(evaluate_hand(&cards), HandRank::Straight);
    }

    #[test]
    fn test_flush() {
        let cards = vec![
            card(Two, Clubs),
            card(Four, Clubs),
            card(Six, Clubs),
            card(Eight, Clubs),
            card(Ten, Clubs),
            card(King, Diamonds),
            card(Ace, Hearts),
        ];
        assert_eq!(evaluate_hand(&cards), HandRank::Flush);
    }

    #[test]
    fn test_full_house() {
        let cards = vec![
            card(Two, Clubs),
            card(Two, Diamonds),
            card(Two, Hearts),
            card(Three, Spades),
            card(Three, Clubs),
            card(King, Diamonds),
            card(Ace, Hearts),
        ];
        assert_eq!(evaluate_hand(&cards), HandRank::FullHouse);
    }

    #[test]
    fn test_four_of_a_kind() {
        let cards = vec![
            card(Two, Clubs),
            card(Two, Diamonds),
            card(Two, Hearts),
            card(Two, Spades),
            card(Three, Clubs),
            card(King, Diamonds),
            card(Ace, Hearts),
        ];
        assert_eq!(evaluate_hand(&cards), HandRank::FourOfAKind);
    }

    #[test]
    fn test_straight_flush() {
        let cards = vec![
            card(Two, Clubs),
            card(Three, Clubs),
            card(Four, Clubs),
            card(Five, Clubs),
            card(Six, Clubs),
            card(King, Diamonds),
            card(Ace, Hearts),
        ];
        assert_eq!(evaluate_hand(&cards), HandRank::StraightFlush);
    }

    #[test]
    fn test_royal_flush() {
        let cards = vec![
            card(Ten, Clubs),
            card(Jack, Clubs),
            card(Queen, Clubs),
            card(King, Clubs),
            card(Ace, Clubs),
            card(Two, Diamonds),
            card(Three, Hearts),
        ];
        assert_eq!(evaluate_hand(&cards), HandRank::RoyalFlush);
    }
}
