use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{
    AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
};
use tracing::debug;

const MAX_MESSAGE_SIZE: usize = 64 * 1024;

pub async fn read_message<T: DeserializeOwned, R: AsyncRead + Unpin>(
    stream: &mut BufReader<R>,
) -> Result<T> {
    let mut line = String::new();
    let mut limited = stream.take(MAX_MESSAGE_SIZE as u64);
    limited.read_line(&mut line).await.context("failed to read line")?;
    if line.len() >= MAX_MESSAGE_SIZE {
        return Err(anyhow::anyhow!("message exceeds maximum size of {} bytes", MAX_MESSAGE_SIZE));
    }
    debug!("received line: {}", line.trim());
    let msg: T = serde_json::from_str(&line).context("failed to parse JSON")?;
    Ok(msg)
}

/// Writes a newline-delimited JSON message to the stream.
pub async fn write_message<T: Serialize, W: AsyncWrite + Unpin>(
    stream: &mut BufWriter<W>,
    msg: &T,
) -> Result<()> {
    let json = serde_json::to_string(msg).context("failed to serialize message")?;
    debug!("sending line: {}", json);
    stream.write_all(json.as_bytes()).await.context("failed to write data")?;
    stream.write_all(b"\n").await.context("failed to write newline")?;
    stream.flush().await.context("failed to flush")?;
    Ok(())
}
