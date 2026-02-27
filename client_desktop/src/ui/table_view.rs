use super::cards::CardWidget;
use super::chips::ChipStackWidget;
use crate::connection::Connection;
use anyhow::anyhow;
use egui::{Color32, Pos2, Rect, Stroke, Ui};
use game_engine::{ActionKind, Card, Pot, Street};
use server::protocol::messages::{HandState as ServerHandState, Message};
use std::sync::Arc;
use tokio::runtime::Runtime;

const DEFAULT_STACK_SIZE: u64 = 1500;
const DEFAULT_BET_AMOUNT: u64 = 100;
const DEFAULT_RAISE_AMOUNT: u64 = 200;

pub struct TableView {
    pub player_seat: u8,
    pub table_id: game_engine::TableId,
    pub hand_id: Option<game_engine::HandId>,
    pub community_cards: Vec<Card>,
    pub pot: Pot,
    pub current_bets: [u64; 2],
    pub player_stacks: [u64; 2],
    pub button_position: u8,
    pub current_street: Street,
    pub acting_seat: Option<u8>,
    pub hole_cards: Vec<Card>,
    pub connection: Option<Connection>,
    pub error_message: Option<String>,
    pub time_remaining_ms: Option<u64>,
    pub time_remaining_received: Option<f64>,
    pub last_sent_action: Option<ActionKind>,
    pub notification: Option<String>,
    runtime: Arc<Runtime>,
}

impl TableView {
    pub fn new(
        player_seat: u8,
        table_id: game_engine::TableId,
        connection: Option<Connection>,
        runtime: Arc<Runtime>,
    ) -> Self {
        Self {
            player_seat,
            table_id,
            hand_id: None,
            community_cards: Vec::new(),
            pot: Pot { main: 0, side_pots: vec![] },
            current_bets: [0, 0],
            player_stacks: [DEFAULT_STACK_SIZE, DEFAULT_STACK_SIZE],
            button_position: 0,
            current_street: Street::PreFlop,
            acting_seat: None,
            hole_cards: Vec::new(),
            connection,
            error_message: None,
            time_remaining_ms: None,
            time_remaining_received: None,
            last_sent_action: None,
            notification: None,
            runtime,
        }
    }

    pub fn set_connection(&mut self, connection: Connection) {
        self.connection = Some(connection);
    }

    pub fn update_from_hand_state(&mut self, hand_state: &ServerHandState) {
        self.notification = None;
        self.hand_id = Some(hand_state.hand_id);
        self.community_cards = hand_state.community_cards.clone();
        self.pot = hand_state.pot.clone();
        self.current_bets = hand_state.current_bets;
        self.current_street = hand_state.current_street;
        self.player_stacks = hand_state.player_stacks;
        self.button_position = hand_state.button_position;
        self.acting_seat = hand_state.acting_seat;
        // hole_cards are already filtered for this player by server
        self.hole_cards = hand_state.hole_cards.clone();
        self.time_remaining_ms = Some(hand_state.time_remaining_ms);
        self.time_remaining_received = None; // will be set when UI updates

        // Detect timeout fold
        if let Some(last_action) = hand_state.actions.last() {
            if last_action.kind == ActionKind::Fold {
                let fold_seat = last_action.seat;
                let is_our_fold = fold_seat == self.player_seat;
                let was_manual = self.last_sent_action == Some(ActionKind::Fold);
                if is_our_fold && !was_manual {
                    self.notification = Some("You folded due to timeout".to_string());
                } else if !is_our_fold {
                    self.notification = Some(format!("Player {} folded due to timeout", fold_seat));
                }
                // Clear last_sent_action after processing
                self.last_sent_action = None;
            }
        }
    }

    fn current_time_remaining_ms(&mut self, ui_time: f64) -> Option<u64> {
        let remaining = self.time_remaining_ms?;
        if let Some(received) = self.time_remaining_received {
            let elapsed_secs = ui_time - received;
            let elapsed_ms = (elapsed_secs * 1000.0).round() as u64;
            if elapsed_ms >= remaining {
                return Some(0);
            }
            Some(remaining - elapsed_ms)
        } else {
            self.time_remaining_received = Some(ui_time);
            Some(remaining)
        }
    }

    fn handle_action(
        &mut self,
        kind: ActionKind,
        amount: Option<u64>,
    ) -> Result<(), anyhow::Error> {
        let hand_id = self.hand_id.ok_or_else(|| anyhow!("no active hand"))?;
        let conn = self.connection.as_mut().ok_or_else(|| anyhow!("no connection"))?;
        self.error_message = None;
        self.last_sent_action = Some(kind);
        let response = self.runtime.block_on(async {
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
                current_bets,
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
                    current_bets,
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
        ui.horizontal(|ui| {
            ui.heading("Poker Table");
            ui.label(format!("({})", self.table_id.as_str()));
        });
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
        // Draw acting seat and time remaining
        if let Some(seat) = self.acting_seat {
            let ui_time = ui.input(|i| i.time);
            if let Some(remaining_ms) = self.current_time_remaining_ms(ui_time) {
                let remaining_secs = remaining_ms as f32 / 1000.0;
                let color = if remaining_secs > 10.0 {
                    Color32::GREEN
                } else if remaining_secs > 5.0 {
                    Color32::YELLOW
                } else {
                    Color32::RED
                };
                ui.colored_label(
                    color,
                    format!("Acting: Player {} ({} seconds remaining)", seat, remaining_secs),
                );
            } else {
                ui.label(format!("Acting: Player {}", seat));
            }
        }

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
        // Show notification (e.g., timeout fold)
        if let Some(notification) = &self.notification {
            ui.colored_label(Color32::BLUE, notification);
        }

        // Action buttons
        ui.horizontal(|ui| {
            let is_my_turn = self.acting_seat == Some(self.player_seat);
            ui.set_enabled(is_my_turn);

            let my_bet = self.current_bets.get(self.player_seat as usize).copied().unwrap_or(0);
            let opponent_bet =
                self.current_bets.get(1 - self.player_seat as usize).copied().unwrap_or(0);
            let call_amount = opponent_bet.saturating_sub(my_bet);
            let my_stack = self.player_stacks.get(self.player_seat as usize).copied().unwrap_or(0);
            let max_call = my_stack.min(call_amount);

            if ui.button("Fold").clicked() {
                let _ = self.handle_action(ActionKind::Fold, None);
            }

            let can_check = call_amount == 0;
            ui.set_enabled(is_my_turn && can_check);
            if ui.button("Check").clicked() {
                let _ = self.handle_action(ActionKind::Check, None);
            }
            ui.set_enabled(is_my_turn);

            let call_button_label = if call_amount == 0 {
                "Check".to_string()
            } else {
                format!("Call ({} chips)", max_call)
            };
            if ui.button(&call_button_label).clicked() {
                if call_amount == 0 {
                    let _ = self.handle_action(ActionKind::Check, None);
                } else {
                    let _ = self.handle_action(ActionKind::Call, Some(max_call));
                }
            }

            let can_bet = my_bet == 0 && my_stack > 0;
            ui.set_enabled(is_my_turn && can_bet);
            if ui.button("Bet").clicked() {
                let bet_amount = my_stack.min(DEFAULT_BET_AMOUNT);
                let _ = self.handle_action(ActionKind::Bet, Some(bet_amount));
            }
            ui.set_enabled(is_my_turn);

            let can_raise = call_amount > 0 && my_stack > call_amount;
            ui.set_enabled(is_my_turn && can_raise);
            if ui.button("Raise").clicked() {
                let raise_increment =
                    DEFAULT_RAISE_AMOUNT.min(my_stack.saturating_sub(call_amount));
                let raise_amount = call_amount.saturating_add(raise_increment);
                let _ = self.handle_action(ActionKind::Raise, Some(raise_amount));
            }
        });
    }
}
