use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::protocol::messages::Message;
use game_engine::{Seat, TableId};

/// Sender for sending messages to a specific connection.
pub type ConnectionSender = mpsc::UnboundedSender<Message>;

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
    /// If the seat is not registered, the message is silently dropped.
    pub async fn send_to_seat(&self, table_id: &TableId, seat: Seat, msg: Message) {
        let inner = self.inner.lock().await;
        if let Some(tx) = inner.connections.get(&(table_id.clone(), seat)) {
            let _ = tx.send(msg);
        }
    }

    /// Broadcast a message to both seats at a table.
    /// If a seat is not registered, the message is not sent to that seat.
    pub async fn broadcast_to_table(
        &self,
        table_id: &TableId,
        msg_factory: impl Fn(Seat) -> Message,
    ) {
        let inner = self.inner.lock().await;
        for seat in [0, 1] {
            if let Some(tx) = inner.connections.get(&(table_id.clone(), seat)) {
                let msg = msg_factory(seat);
                let _ = tx.send(msg);
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
