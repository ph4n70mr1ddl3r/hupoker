use super::cards::CardWidget;
use super::chips::ChipStackWidget;
use crate::connection::Connection;
use anyhow::anyhow;
use egui::{Color32, Pos2, Rect, Stroke, Ui};
use game_engine::{ActionKind, Card, Pot, Street};
use server::protocol::messages::{HandState as ServerHandState, Message};
use tokio::runtime::Runtime;

pub struct TableView {
    pub player_seat: u8,
    #[allow(dead_code)]
    pub table_id: game_engine::TableId,
    pub hand_id: Option<game_engine::HandId>,
    pub community_cards: Vec<Card>,
    pub pot: Pot,
    pub player_stacks: [u64; 2],
    pub button_position: u8,
    pub current_street: Street,
    pub acting_seat: Option<u8>,
    pub hole_cards: Vec<Card>, // for the current player
    pub connection: Option<Connection>,
    pub error_message: Option<String>,
}

impl TableView {
    pub fn new(
        player_seat: u8,
        table_id: game_engine::TableId,
        connection: Option<Connection>,
    ) -> Self {
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
            connection,
            error_message: None,
        }
    }

    pub fn set_connection(&mut self, connection: Connection) {
        self.connection = Some(connection);
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

    fn handle_action(
        &mut self,
        kind: ActionKind,
        amount: Option<u64>,
    ) -> Result<(), anyhow::Error> {
        let hand_id = self.hand_id.ok_or_else(|| anyhow!("no active hand"))?;
        let conn = self.connection.as_mut().ok_or_else(|| anyhow!("no connection"))?;
        let rt = Runtime::new()?;
        self.error_message = None;
        let response = rt.block_on(async {
            conn.send_action(hand_id, kind, amount).await?;
            conn.receive_message().await
        })?;
        match response {
            Message::HandState {
                version: _,
                hand_id,
                table_id,
                hole_cards,
                community_cards,
                pot,
                current_street,
                actions,
                player_stacks,
                button_position,
                last_action_time,
                acting_seat,
                time_remaining_ms,
            } => {
                let hand_state = ServerHandState {
                    hand_id,
                    table_id,
                    hole_cards,
                    community_cards,
                    pot,
                    current_street,
                    actions,
                    player_stacks,
                    button_position,
                    last_action_time,
                    acting_seat,
                    time_remaining_ms,
                };
                self.update_from_hand_state(&hand_state);
                Ok(())
            }
            Message::Error { code, message, .. } => {
                self.error_message = Some(format!("server error {}: {}", code, message));
                Err(anyhow!("server error {}: {}", code, message))
            }
            _ => {
                self.error_message = Some("unexpected response".to_string());
                Err(anyhow!("unexpected response"))
            }
        }
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
            let rect = Rect::from_min_size(
                Pos2::new(x, community_center.y - card_height * 0.5),
                egui::Vec2::new(card_width, card_height),
            );
            CardWidget::draw(ui, card, rect);
            x += card_width + spacing;
        }

        // Draw pot (below community cards)
        let pot_total = self.pot.total();
        let pot_rect = Rect::from_min_size(
            Pos2::new(community_center.x - 40.0, community_center.y + card_height * 0.5 + 20.0),
            egui::Vec2::new(80.0, 80.0),
        );
        ChipStackWidget::draw_pot(ui, pot_total, pot_rect);

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
                let card_width = 40.0;
                let card_height = 60.0;
                let spacing = 10.0;
                for card in &self.hole_cards {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::Vec2::new(card_width, card_height),
                        egui::Sense::hover(),
                    );
                    CardWidget::draw(ui, card, rect);
                    ui.add_space(spacing);
                }
            });
        }

        // Show error message if any
        if let Some(err) = &self.error_message {
            ui.colored_label(Color32::RED, err);
        }

        // Action buttons
        ui.horizontal(|ui| {
            let is_my_turn = self.acting_seat == Some(self.player_seat);
            ui.set_enabled(is_my_turn);
            if ui.button("Fold").clicked() {
                let _ = self.handle_action(ActionKind::Fold, None);
            }
            if ui.button("Check").clicked() {
                let _ = self.handle_action(ActionKind::Check, None);
            }
            if ui.button("Call").clicked() {
                // TODO: need amount to call
                let _ = self.handle_action(ActionKind::Call, Some(100));
            }
            if ui.button("Bet").clicked() {
                // TODO: need bet amount
                let _ = self.handle_action(ActionKind::Bet, Some(100));
            }
            if ui.button("Raise").clicked() {
                // TODO: need raise amount
                let _ = self.handle_action(ActionKind::Raise, Some(200));
            }
        });
    }
}
