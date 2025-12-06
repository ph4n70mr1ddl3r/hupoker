use game_engine::{Player, Table, TableId};

pub struct TableManager {
    tables: Vec<Table>,
}

impl TableManager {
    pub fn new() -> Self {
        Self { tables: Vec::new() }
    }

    pub fn add_table(&mut self, table: Table) {
        self.tables.push(table);
    }

    pub fn get_table(&self, id: &TableId) -> Option<&Table> {
        self.tables.iter().find(|t| &t.id == id)
    }

    pub fn get_table_mut(&mut self, id: &TableId) -> Option<&mut Table> {
        self.tables.iter_mut().find(|t| &t.id == id)
    }

    /// Attempts to occupy a seat at a table.
    /// Returns Ok(()) on success, Err(message) otherwise.
    pub fn occupy_seat(
        &mut self,
        table_id: &TableId,
        seat: u8,
        player: Player,
    ) -> Result<(), String> {
        let table = self.get_table_mut(table_id).ok_or_else(|| "table not found".to_string())?;
        if seat >= 2 {
            return Err("seat must be 0 or 1".to_string());
        }
        if table.seats[seat as usize].is_some() {
            return Err("seat already occupied".to_string());
        }
        // Ensure player seat matches
        if player.seat != seat {
            return Err("player seat does not match".to_string());
        }
        table.seats[seat as usize] = Some(player);
        Ok(())
    }
}
