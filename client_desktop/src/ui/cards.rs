use egui::{Color32, Rect, Stroke, Ui};
use game_engine::{Card, Rank, Suit};

/// Widget that renders a single card.
pub struct CardWidget;

impl CardWidget {
    /// Draw a card at the given position with given size.
    pub fn draw(ui: &mut Ui, card: &Card, rect: Rect) {
        let color = match card.suit {
            Suit::Clubs | Suit::Spades => Color32::BLACK,
            Suit::Diamonds | Suit::Hearts => Color32::RED,
            _ => Color32::GRAY,
        };

        // Card background
        ui.painter().rect(rect, 5.0, Color32::WHITE, Stroke::new(1.0, color));

        // Rank and suit symbol
        let rank_str = rank_symbol(card.rank);
        let suit_str = suit_symbol(card.suit);

        let text = format!("{}\n{}", rank_str, suit_str);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::monospace(14.0),
            color,
        );
    }
}

fn rank_symbol(rank: Rank) -> &'static str {
    match rank {
        Rank::Two => "2",
        Rank::Three => "3",
        Rank::Four => "4",
        Rank::Five => "5",
        Rank::Six => "6",
        Rank::Seven => "7",
        Rank::Eight => "8",
        Rank::Nine => "9",
        Rank::Ten => "10",
        Rank::Jack => "J",
        Rank::Queen => "Q",
        Rank::King => "K",
        Rank::Ace => "A",
        _ => "?",
    }
}

fn suit_symbol(suit: Suit) -> &'static str {
    match suit {
        Suit::Clubs => "♣",
        Suit::Diamonds => "♦",
        Suit::Hearts => "♥",
        Suit::Spades => "♠",
        _ => "?",
    }
}
