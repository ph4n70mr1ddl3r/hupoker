use rand_chacha::ChaCha12Rng;
use rand_core::{RngCore, SeedableRng};
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
        let mut cards: Vec<Card> = Vec::with_capacity(DECK_SIZE);
        for &rank in &[
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
        ] {
            for &suit in &[Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                cards.push(Card { rank, suit });
            }
        }
        // Fisher-Yates shuffle using RngCore
        for i in (1..cards.len()).rev() {
            let j = (rng.next_u32() as usize) % (i + 1);
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
