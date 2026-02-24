use anyhow::{Context, Result};
use game_engine::{ActionKind, HandId};
use server::protocol::messages::Message;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing::debug;

const READ_TIMEOUT_SECS: u64 = 30;

pub struct Connection {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>,
}

impl Connection {
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (read_half, write_half) = stream.into_split();
        let reader = BufReader::new(read_half);
        let writer = BufWriter::new(write_half);
        Ok(Self { reader, writer })
    }

    pub async fn send_message(&mut self, msg: &Message) -> Result<()> {
        let json = serde_json::to_string(msg).context("failed to serialize message")?;
        debug!("sending: {}", json);
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn receive_message(&mut self) -> Result<Message> {
        let mut line = String::new();
        timeout(Duration::from_secs(READ_TIMEOUT_SECS), self.reader.read_line(&mut line))
            .await
            .context("read timeout")?
            .context("failed to read line")?;
        debug!("received: {}", line.trim());
        let msg: Message = serde_json::from_str(&line).context("failed to parse JSON")?;
        Ok(msg)
    }

    pub async fn handshake(&mut self, client_name: &str, client_version: &str) -> Result<()> {
        // Send ClientHello
        let client_hello = Message::ClientHello {
            version: "1.0".to_string(),
            client_name: client_name.to_string(),
            client_version: client_version.to_string(),
        };
        self.send_message(&client_hello).await?;

        // Expect ServerHello
        let response = self.receive_message().await?;
        match response {
            Message::ServerHello { version, status, .. } => {
                if !version.starts_with("1.") {
                    anyhow::bail!("unsupported server version: {}", version);
                }
                if status != "accepted" {
                    anyhow::bail!("server rejected connection: {}", status);
                }
                Ok(())
            }
            Message::Error { code, message, .. } => {
                anyhow::bail!("server error {}: {}", code, message);
            }
            _ => anyhow::bail!("unexpected response to ClientHello"),
        }
    }

    pub async fn join_table(
        &mut self,
        table_id: game_engine::TableId,
        seat: u8,
    ) -> Result<(server::protocol::messages::TableState, Vec<server::protocol::messages::HandState>)>
    {
        let join = Message::JoinTable { version: "1.0".to_string(), table_id, seat };
        self.send_message(&join).await?;

        let mut hand_states = Vec::new();
        let table_state = loop {
            let response = self.receive_message().await?;
            match response {
                Message::TableState { version: _, table_id, seats, config, current_hand_id } => {
                    break server::protocol::messages::TableState {
                        table_id,
                        seats,
                        config,
                        current_hand_id,
                    };
                }
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
                    hand_states.push(server::protocol::messages::HandState {
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
                    });
                }
                Message::Error { code, message, .. } => {
                    anyhow::bail!("join table error {}: {}", code, message);
                }
                _ => anyhow::bail!("unexpected response to JoinTable"),
            }
        };
        Ok((table_state, hand_states))
    }

    pub async fn send_action(
        &mut self,
        hand_id: HandId,
        kind: ActionKind,
        amount: Option<u64>,
    ) -> Result<()> {
        let action = Message::Action { version: "1.0".to_string(), hand_id, kind, amount };
        self.send_message(&action).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    #[ignore]
    async fn test_connect_and_join() {
        // This test requires a running server; we'll spawn one in the same process
        // For now, ignore.
    }
}
