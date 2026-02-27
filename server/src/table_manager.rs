use chrono::{Duration, Utc};
use game_engine::{HandId, Player, Seat, Table, TableId, NUM_SEATS};
use std::collections::HashMap;

pub struct TableManager {
    tables: HashMap<TableId, Table>,
}

impl TableManager {
    pub fn new() -> Self {
        Self { tables: HashMap::new() }
    }

    pub fn add_table(&mut self, table: Table) {
        self.tables.insert(table.id.clone(), table);
    }

    pub fn get_table(&self, id: &TableId) -> Option<&Table> {
        self.tables.get(id)
    }

    pub fn get_table_mut(&mut self, id: &TableId) -> Option<&mut Table> {
        self.tables.get_mut(id)
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
        if seat as usize >= NUM_SEATS {
            return Err(format!("seat must be 0..{}", NUM_SEATS - 1));
        }
        // Check if seat already occupied
        if let Some(existing_player) = &table.seats[seat as usize] {
            // Seat occupied; check if disconnected and within reconnection timeout
            if let Some(disconnected_at) = existing_player.disconnected_at {
                let elapsed = Utc::now() - disconnected_at;
                let timeout =
                    chrono::Duration::seconds(table.config.reconnection_timeout_secs as i64);
                if elapsed < timeout {
                    // Allow reconnection: replace player, clear disconnected_at
                    // Ensure player seat matches
                    if player.seat != seat {
                        return Err("player seat does not match".to_string());
                    }
                    // Replace player (keeping stack? we keep existing player's stack?)
                    // For now, keep existing stack and update connection_id, clear disconnected_at
                    let mut new_player = player;
                    new_player.stack = existing_player.stack;
                    new_player.disconnected_at = None;
                    table.seats[seat as usize] = Some(new_player);
                    return Ok(());
                }
                // Timeout expired; remove the player (seat becomes empty)
                table.seats[seat as usize] = None;
            } else {
                // Player connected, seat occupied
                return Err("seat already occupied".to_string());
            }
        }

        // Ensure player seat matches
        if player.seat != seat {
            return Err("player seat does not match".to_string());
        }
        table.seats[seat as usize] = Some(player);
        Ok(())
    }

    /// Start a new hand at the specified table.
    /// Requires both seats occupied and no current hand.
    /// `seed` is a 32-byte random seed for the deck.
    /// Returns the new HandId on success.
    pub fn start_hand(&mut self, table_id: &TableId, seed: [u8; 32]) -> Result<HandId, String> {
        let table = self.get_table_mut(table_id).ok_or_else(|| "table not found".to_string())?;
        table.start_hand(seed).map_err(|e| e.to_string())
    }

    /// Start a new hand with a pre-generated HandId.
    /// This allows audit logging to happen atomically before the hand starts.
    pub fn start_hand_with_id(
        &mut self,
        table_id: &TableId,
        seed: [u8; 32],
        hand_id: game_engine::HandId,
    ) -> Result<game_engine::HandId, String> {
        let table = self.get_table_mut(table_id).ok_or_else(|| "table not found".to_string())?;
        table.start_hand_with_id(seed, hand_id).map_err(|e| e.to_string())
    }

    /// Mark a player as disconnected at the given seat.
    /// Returns true if the seat was occupied and the player was marked.
    pub fn mark_disconnected(&mut self, table_id: &TableId, seat: u8) -> bool {
        if seat as usize >= NUM_SEATS {
            return false;
        }
        let table = match self.get_table_mut(table_id) {
            Some(table) => table,
            None => return false,
        };
        if let Some(player) = &mut table.seats[seat as usize] {
            player.disconnected_at = Some(Utc::now());
            true
        } else {
            false
        }
    }

    /// Check all tables for players who have exceeded their action timeout.
    /// Returns a vector of (table_id, seat) for each player that should auto‑fold.
    pub fn check_action_timeouts(&self) -> Vec<(TableId, Seat)> {
        let mut timed_out = Vec::new();
        let now = Utc::now();

        for table in self.tables.values() {
            // Only check tables with an active hand
            let hand = match &table.current_hand {
                Some(h) => h,
                None => continue,
            };
            // Determine whose turn it is
            let acting_seat = match hand.betting.acting_seat() {
                Some(seat) => seat,
                None => continue,
            };
            // Get the timeout duration from table config
            let timeout = Duration::seconds(table.config.action_timeout_secs as i64);
            // Check if last_action_time exists and is older than timeout
            if let Some(last_action_time) = hand.last_action_time {
                let elapsed = now - last_action_time;
                if elapsed > timeout {
                    timed_out.push((table.id.clone(), acting_seat));
                }
            }
        }
        timed_out
    }
}

impl Default for TableManager {
    fn default() -> Self {
        Self::new()
    }
}
