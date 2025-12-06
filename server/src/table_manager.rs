use game_engine::{Table, TableId};

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
}