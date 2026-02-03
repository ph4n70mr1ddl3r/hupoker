use rand::Rng;
use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;
use serde::{Deserialize, Serialize};

const DECK_SIZE: usize = 52;

/// A shuffled 52‑card deck.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    cards: Vec<super::Card>,
}

impl Deck {
    pub fn new(seed: [u8; 32]) -> Self {
        use super::{Card, Rank, Suit};
        let mut rng = ChaCha12Rng::from_seed(seed);
        let suits = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
        let ranks = [
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
            Rank::Ace,
        ];
        let mut cards: Vec<Card> = ranks
            .iter()
            .flat_map(|&rank| suits.iter().map(move |&suit| Card { rank, suit }))
            .collect();
        // Fisher-Yates shuffle
        for i in (1..cards.len()).rev() {
            let j = rng.gen_range(0..=i);
            cards.swap(i, j);
        }
        Self { cards }
    }

    pub fn draw(&mut self) -> Option<super::Card> {
        self.cards.pop()
    }

    pub fn remaining(&self) -> usize {
        self.cards.len()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.cards.len() > DECK_SIZE {
            return Err(format!("deck has too many cards: {}", self.cards.len()));
        }
        Ok(())
    }
}
