use super::Card;
use super::Rank;
use super::Suit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandRank {
    HighCard = 0,
    OnePair = 1,
    TwoPair = 2,
    ThreeOfAKind = 3,
    Straight = 4,
    Flush = 5,
    FullHouse = 6,
    FourOfAKind = 7,
    StraightFlush = 8,
    RoyalFlush = 9,
}

pub type HandScore = u64;

fn rank_value(rank: Rank) -> u64 {
    rank.value() as u64
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
    let mut values: Vec<u8> = cards.iter().map(|c| c.rank.value()).collect();
    values.sort_unstable();
    values.dedup();
    // handle ace low straight (A-2-3-4-5)
    if values.contains(&14) {
        let mut low_values: Vec<u8> = values.iter().map(|&v| if v == 14 { 1 } else { v }).collect();
        low_values.sort_unstable();
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
    counts.sort_by_key(|&(rank, count)| (count, rank.value()));
    counts.reverse();
    counts
}

fn evaluate_5card_hand(cards: &[Card]) -> HandRank {
    if cards.len() != 5 {
        return HandRank::HighCard;
    }
    let counts = count_ranks(cards);
    let is_flush = is_flush(cards);
    let is_straight = is_straight(cards);

    if is_flush && is_straight {
        let mut values: Vec<u8> = cards.iter().map(|c| c.rank.value()).collect();
        values.sort_unstable();
        if values == [10, 11, 12, 13, 14] {
            return HandRank::RoyalFlush;
        }
        return HandRank::StraightFlush;
    }
    if counts.is_empty() {
        return HandRank::HighCard;
    }
    match counts[0].1 {
        4 => HandRank::FourOfAKind,
        3 => {
            if counts.get(1).map(|c| c.1 >= 2).unwrap_or(false) {
                HandRank::FullHouse
            } else {
                HandRank::ThreeOfAKind
            }
        }
        2 => {
            if counts.get(1).map(|c| c.1 == 2).unwrap_or(false) {
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

fn evaluate_5card_score(cards: &[Card]) -> HandScore {
    if cards.len() != 5 {
        return 0;
    }
    let counts = count_ranks(cards);
    let is_flush = is_flush(cards);
    let is_straight_val = is_straight(cards);
    let mut values: Vec<u8> = cards.iter().map(|c| c.rank.value()).collect();
    values.sort_unstable();
    values.reverse();
    let values_iter = values.iter();

    if is_flush && is_straight_val {
        if values == [14, 13, 12, 11, 10] {
            return (HandRank::RoyalFlush as HandScore) << 20;
        }
        let high =
            if values.contains(&14) && values.contains(&5) { 5u64 } else { values[0] as u64 };
        return ((HandRank::StraightFlush as HandScore) << 20) | (high << 16);
    }

    if counts.is_empty() {
        return (HandRank::HighCard as HandScore) << 20;
    }

    if counts[0].1 == 4 {
        let quad_rank = rank_value(counts[0].0);
        let kicker = counts.get(1).map(|c| rank_value(c.0)).unwrap_or(0);
        return ((HandRank::FourOfAKind as HandScore) << 20) | (quad_rank << 16) | (kicker << 12);
    }

    if counts[0].1 == 3 && counts.get(1).map(|c| c.1 >= 2).unwrap_or(false) {
        let trips_rank = rank_value(counts[0].0);
        let pair_rank = rank_value(counts[1].0);
        return ((HandRank::FullHouse as HandScore) << 20) | (trips_rank << 16) | (pair_rank << 12);
    }

    if is_flush {
        let score = values_iter.fold(0u64, |acc, &v| (acc << 4) | (v as u64));
        return ((HandRank::Flush as HandScore) << 20) | score;
    }

    if is_straight_val {
        let high =
            if values.contains(&14) && values.contains(&5) { 5u64 } else { values[0] as u64 };
        return ((HandRank::Straight as HandScore) << 20) | (high << 16);
    }

    if counts[0].1 == 3 {
        let trips_rank = rank_value(counts[0].0);
        let mut kickers = 0u64;
        if counts.len() > 1 {
            for c in &counts[1..] {
                kickers = (kickers << 4) | rank_value(c.0);
            }
        }
        return ((HandRank::ThreeOfAKind as HandScore) << 20) | (trips_rank << 16) | kickers;
    }

    if counts[0].1 == 2 && counts.get(1).map(|c| c.1 == 2).unwrap_or(false) {
        let high_pair = rank_value(counts[0].0).max(rank_value(counts[1].0));
        let low_pair = rank_value(counts[0].0).min(rank_value(counts[1].0));
        let kicker = counts.get(2).map(|c| rank_value(c.0)).unwrap_or(0);
        return ((HandRank::TwoPair as HandScore) << 20)
            | (high_pair << 16)
            | (low_pair << 12)
            | (kicker << 8);
    }

    if counts[0].1 == 2 {
        let pair_rank = rank_value(counts[0].0);
        let mut kickers = 0u64;
        if counts.len() > 1 {
            for c in &counts[1..] {
                kickers = (kickers << 4) | rank_value(c.0);
            }
        }
        return ((HandRank::OnePair as HandScore) << 20) | (pair_rank << 16) | kickers;
    }

    let score = values_iter.fold(0u64, |acc, &v| (acc << 4) | (v as u64));
    ((HandRank::HighCard as HandScore) << 20) | score
}

pub fn evaluate_hand(cards: &[Card]) -> HandRank {
    if cards.len() < 5 {
        return HandRank::HighCard;
    }
    let n = cards.len();
    let mut best = HandRank::HighCard;
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

pub fn evaluate_hand_score(cards: &[Card]) -> HandScore {
    if cards.len() < 5 {
        return 0;
    }
    let n = cards.len();
    let mut best_score: HandScore = 0;
    for i in 0..n {
        for j in i + 1..n {
            for k in j + 1..n {
                for l in k + 1..n {
                    for m in l + 1..n {
                        let combo = [cards[i], cards[j], cards[k], cards[l], cards[m]];
                        let score = evaluate_5card_score(&combo);
                        if score > best_score {
                            best_score = score;
                        }
                    }
                }
            }
        }
    }
    best_score
}

pub fn score_to_rank(score: HandScore) -> HandRank {
    match (score >> 20) as u8 {
        0 => HandRank::HighCard,
        1 => HandRank::OnePair,
        2 => HandRank::TwoPair,
        3 => HandRank::ThreeOfAKind,
        4 => HandRank::Straight,
        5 => HandRank::Flush,
        6 => HandRank::FullHouse,
        7 => HandRank::FourOfAKind,
        8 => HandRank::StraightFlush,
        9 => HandRank::RoyalFlush,
        _ => HandRank::HighCard,
    }
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
