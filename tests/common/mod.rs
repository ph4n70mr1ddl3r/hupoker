use anyhow::Result;
use server::protocol::Message;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

pub struct TestClient {
    pub reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    pub writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>,
}

impl TestClient {
    pub async fn connect(addr: std::net::SocketAddr) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (read_half, write_half) = stream.into_split();
        let reader = BufReader::new(read_half);
        let writer = BufWriter::new(write_half);
        Ok(Self { reader, writer })
    }

    pub async fn send(&mut self, msg: &serde_json::Value) -> Result<()> {
        let line = serde_json::to_string(msg)? + "\n";
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Message> {
        let mut line = String::new();
        self.reader.read_line(&mut line).await?;
        let msg: Message = serde_json::from_str(&line)?;
        Ok(msg)
    }

    pub async fn recv_timeout(&mut self, dur: Duration) -> Result<Message> {
        timeout(dur, self.recv()).await?
    }
}
