use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tracing::warn;

use crate::protocol::messages::Message;
use game_engine::{Seat, TableId};

/// Sender for sending messages to a specific connection.
pub type ConnectionSender = mpsc::Sender<Message>;

/// Inner state of the connection manager, protected by a mutex.
struct ConnectionManagerInner {
    /// Mapping from table ID and seat to the sender for that connection.
    connections: HashMap<(TableId, Seat), ConnectionSender>,
}

/// Manages active connections and allows sending messages to specific seats or broadcasting to tables.
#[derive(Clone)]
pub struct ConnectionManager {
    inner: Arc<Mutex<ConnectionManagerInner>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(ConnectionManagerInner { connections: HashMap::new() })) }
    }

    /// Register a new connection for a given table and seat.
    /// The sender will be used to send messages to this connection.
    pub async fn register(&self, table_id: TableId, seat: Seat, sender: ConnectionSender) {
        let mut inner = self.inner.lock().await;
        inner.connections.insert((table_id, seat), sender);
    }

    /// Unregister a connection (e.g., when the client disconnects).
    pub async fn unregister(&self, table_id: &TableId, seat: Seat) {
        let mut inner = self.inner.lock().await;
        inner.connections.remove(&(table_id.clone(), seat));
    }

    /// Send a message to a specific seat at a specific table.
    /// If the seat is not registered or the sender is closed, the message is silently dropped
    /// and the sender is removed from the map.
    pub async fn send_to_seat(&self, table_id: &TableId, seat: Seat, msg: Message) {
        let tx = {
            let inner = self.inner.lock().await;
            let key = (table_id.clone(), seat);
            inner.connections.get(&key).cloned()
        };
        if let Some(tx) = tx {
            if tx.try_send(msg).is_err() {
                // Sender is closed, remove it from the map
                let mut inner = self.inner.lock().await;
                let key = (table_id.clone(), seat);
                inner.connections.remove(&key);
                warn!("removed broken sender for seat {} at table {}", seat, table_id.as_str());
            }
        } else {
            warn!("no connection for seat {} at table {}", seat, table_id.as_str());
        }
    }

    /// Broadcast a message to both seats at a table.
    /// If a seat is not registered or the sender is closed, the message is not sent to that seat
    /// and the sender is removed from the map.
    pub async fn broadcast_to_table(
        &self,
        table_id: &TableId,
        msg_factory: impl Fn(Seat) -> Message,
    ) {
        // Collect senders to broadcast to while holding the lock
        let senders: Vec<(Seat, ConnectionSender)> = {
            let inner = self.inner.lock().await;
            [0, 1]
                .iter()
                .filter_map(|&seat| {
                    let key = (table_id.clone(), seat);
                    inner.connections.get(&key).map(|tx| (seat, tx.clone()))
                })
                .collect()
        };

        // Send messages without holding the lock
        let mut broken_seats = Vec::new();
        for (seat, tx) in senders {
            let msg = msg_factory(seat);
            if tx.try_send(msg).is_err() {
                broken_seats.push(seat);
                warn!("failed to broadcast to seat {} at table {}", seat, table_id.as_str());
            }
        }

        // Remove broken senders from the map
        if !broken_seats.is_empty() {
            let mut inner = self.inner.lock().await;
            for seat in broken_seats {
                let key = (table_id.clone(), seat);
                inner.connections.remove(&key);
            }
        }
    }

    /// Check if a seat at a table is currently registered (i.e., has an active connection).
    pub async fn is_seat_occupied(&self, table_id: &TableId, seat: Seat) -> bool {
        let inner = self.inner.lock().await;
        inner.connections.contains_key(&(table_id.clone(), seat))
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
