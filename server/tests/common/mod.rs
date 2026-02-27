use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;

pub async fn connect_and_join(
    addr: std::net::SocketAddr,
    seat: u8,
) -> Result<(
    BufReader<tokio::net::tcp::OwnedReadHalf>,
    BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    serde_json::Value,
    Vec<serde_json::Value>,
)> {
    let stream = TcpStream::connect(addr).await?;
    let (read_half, write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut writer = BufWriter::new(write_half);

    let client_hello = serde_json::json!({
        "type": "client_hello",
        "version": "1.0",
        "client_name": "test",
        "client_version": "0.1.0"
    });
    let line = serde_json::to_string(&client_hello)? + "\n";
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await?;

    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let server_hello: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(server_hello["type"], "server_hello");
    assert_eq!(server_hello["status"], "accepted");

    let join_table = serde_json::json!({
        "type": "join_table",
        "version": "1.0",
        "table_id": "table-0",
        "seat": seat
    });
    let line = serde_json::to_string(&join_table)? + "\n";
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await?;

    let mut table_state = None;
    let mut hand_states = Vec::new();
    while table_state.is_none() {
        let mut response = String::new();
        reader.read_line(&mut response).await?;
        let msg: serde_json::Value = serde_json::from_str(&response)?;
        match msg["type"].as_str() {
            Some("table_state") => {
                assert_eq!(msg["table_id"], "table-0");
                table_state = Some(msg);
            }
            Some("hand_state") => {
                hand_states.push(msg);
            }
            _ => panic!("unexpected message type: {}", msg["type"]),
        }
    }

    Ok((reader, writer, table_state.unwrap(), hand_states))
}
