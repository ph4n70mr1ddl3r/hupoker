use egui::{Color32, Pos2, Rect, Shape, Stroke, Ui};
use game_engine::{Card, Pot, Street};
use server::protocol::messages::HandState as ServerHandState;

pub struct TableView {
    pub player_seat: u8,
    pub table_id: game_engine::TableId,
    pub hand_id: Option<game_engine::HandId>,
    pub community_cards: Vec<Card>,
    pub pot: Pot,
    pub player_stacks: [u64; 2],
    pub button_position: u8,
    pub current_street: Street,
    pub acting_seat: Option<u8>,
    pub hole_cards: Vec<Card>, // for the current player
}

impl TableView {
    pub fn new(player_seat: u8, table_id: game_engine::TableId) -> Self {
        Self {
            player_seat,
            table_id,
            hand_id: None,
            community_cards: Vec::new(),
            pot: Pot { main: 0, side_pots: vec![] },
            player_stacks: [1500, 1500],
            button_position: 0,
            current_street: Street::PreFlop,
            acting_seat: None,
            hole_cards: Vec::new(),
        }
    }

    pub fn update_from_hand_state(&mut self, hand_state: &ServerHandState) {
        self.hand_id = Some(hand_state.hand_id);
        self.community_cards = hand_state.community_cards.clone();
        self.pot = hand_state.pot.clone();
        self.current_street = hand_state.current_street;
        self.player_stacks = hand_state.player_stacks;
        self.button_position = hand_state.button_position;
        self.acting_seat = hand_state.acting_seat;
        // hole_cards are already filtered for this player by server
        self.hole_cards = hand_state.hole_cards.clone();
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.heading("Poker Table");
        ui.separator();

        // Draw table background (circle or rectangle)
        let available = ui.available_size();
        let center = Pos2::new(available.x * 0.5, available.y * 0.5);
        let table_radius = available.x.min(available.y) * 0.4;
        ui.painter().circle(
            center,
            table_radius,
            Color32::from_rgb(0, 100, 0),
            Stroke::new(2.0, Color32::from_rgb(0, 150, 0)),
        );

        // Draw community cards area (center)
        let community_center = center;
        let card_width = 40.0;
        let card_height = 60.0;
        let spacing = 10.0;
        let total_width = self.community_cards.len() as f32 * (card_width + spacing) - spacing;
        let mut x = community_center.x - total_width * 0.5;
        for card in &self.community_cards {
            let rect = Rect::from_min_size(Pos2::new(x, community_center.y - card_height * 0.5), egui::Vec2::new(card_width, card_height));
            ui.painter().rect(
                rect,
                5.0,
                Color32::from_rgb(255, 255, 255),
                Stroke::new(1.0, Color32::from_rgb(0, 0, 0)),
            );
            // Draw rank/suit text (simplified)
            let text = format!("{:?}{:?}", card.rank, card.suit);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::monospace(12.0),
                Color32::BLACK,
            );
            x += card_width + spacing;
        }

        // Draw pot
        ui.label(format!("Pot: {}", self.pot.total()));

        // Draw player stacks
        ui.horizontal(|ui| {
            ui.label(format!("Player 0 stack: {}", self.player_stacks[0]));
            ui.label(format!("Player 1 stack: {}", self.player_stacks[1]));
        });

        // Draw button indicator
        ui.label(format!("Button: Player {}", self.button_position));

        // Draw street
        ui.label(format!("Street: {:?}", self.current_street));

        // Draw hole cards (for current player)
        if !self.hole_cards.is_empty() {
            ui.label("Your hole cards:");
            ui.horizontal(|ui| {
                for card in &self.hole_cards {
                    let card_text = format!("{:?}{:?}", card.rank, card.suit);
                    ui.label(card_text);
                }
            });
        }

        // Action buttons (placeholder)
        ui.horizontal(|ui| {
            if ui.button("Fold").clicked() {
                // TODO
            }
            if ui.button("Check").clicked() {
                // TODO
            }
            if ui.button("Call").clicked() {
                // TODO
            }
            if ui.button("Bet").clicked() {
                // TODO
            }
            if ui.button("Raise").clicked() {
                // TODO
            }
        });
    }
}