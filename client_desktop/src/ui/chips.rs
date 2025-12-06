use egui::{Color32, Pos2, Rect, Shape, Stroke, Ui};

/// Widget that renders a stack of chips.
pub struct ChipStackWidget;

impl ChipStackWidget {
    /// Draw a chip stack at the given position with given size.
    /// `amount` is the total chip count.
    pub fn draw(ui: &mut Ui, amount: u64, rect: Rect) {
        // Draw chip stack as overlapping circles
        let center = rect.center();
        let chip_radius = rect.width().min(rect.height()) * 0.5;
        let chip_color = Color32::from_rgb(255, 215, 0); // gold

        // Draw base chip
        ui.painter().circle(center, chip_radius, chip_color, Stroke::new(2.0, Color32::BLACK));

        // Draw amount text
        let text = if amount >= 1_000_000 {
            format!("{}M", amount / 1_000_000)
        } else if amount >= 1_000 {
            format!("{}K", amount / 1_000)
        } else {
            format!("{}", amount)
        };

        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::monospace(12.0),
            Color32::BLACK,
        );
    }

    /// Draw a pot (chip stack with label)
    pub fn draw_pot(ui: &mut Ui, pot_total: u64, rect: Rect) {
        Self::draw(ui, pot_total, rect);

        // Draw "Pot" label below
        let label_pos = Pos2::new(rect.center().x, rect.bottom() + 10.0);
        ui.painter().text(
            label_pos,
            egui::Align2::CENTER_CENTER,
            "Pot",
            egui::FontId::monospace(10.0),
            Color32::GRAY,
        );
    }
}
