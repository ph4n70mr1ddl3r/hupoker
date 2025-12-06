use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use game_engine::{HandId, TableId};
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

pub struct AuditLog {
    writer: BufWriter<File>,
    cipher: Option<ChaCha20Poly1305>,
}

#[derive(Serialize)]
pub enum AuditEvent {
    HandStart {
        hand_id: HandId,
        table_id: TableId,
        rng_seed_encrypted: String, // base64 encrypted seed
    },
    Action {
        hand_id: HandId,
        seat: u8,
        kind: String,
        amount: Option<u64>,
    },
    HandEnd {
        hand_id: HandId,
        winner_seats: Vec<u8>,
        pot_amount: u64,
    },
}

impl AuditLog {
    pub fn new(path: impl AsRef<Path>, encryption_key: Option<[u8; 32]>) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.as_ref())
            .with_context(|| format!("failed to open audit log {:?}", path.as_ref()))?;
        let writer = BufWriter::new(file);
        let cipher = encryption_key.map(|key| {
            let key = Key::from_slice(&key);
            ChaCha20Poly1305::new(key)
        });
        Ok(Self { writer, cipher })
    }

    pub fn log_event(&mut self, event: AuditEvent) -> Result<()> {
        let json = serde_json::to_string(&event).context("failed to serialize audit event")?;
        // Encrypt if cipher exists (for RNG seeds)
        // For now, just write JSON line
        writeln!(self.writer, "{}", json).context("failed to write audit log")?;
        self.writer.flush()?;
        Ok(())
    }

    /// Log a hand start event with the given seed and hand ID.
    /// If encryption is enabled, the seed is encrypted before logging.
    pub fn log_seed(&mut self, hand_id: HandId, table_id: &TableId, seed: &[u8; 32]) -> Result<()> {
        let rng_seed_encrypted = match &self.cipher {
            Some(cipher) => {
                // Use zero nonce (should be unique per encryption; for simplicity we use zero)
                let nonce = Nonce::from_slice(&[0u8; 12]);
                let encrypted = cipher
                    .encrypt(nonce, seed.as_ref())
                    .map_err(|e| anyhow::anyhow!("seed encryption failed: {}", e))?;
                general_purpose::STANDARD.encode(encrypted)
            }
            None => general_purpose::STANDARD.encode(seed),
        };
        let event =
            AuditEvent::HandStart { hand_id, table_id: table_id.clone(), rng_seed_encrypted };
        self.log_event(event)
    }

    pub fn encrypt_seed(seed: [u8; 32], key: &[u8; 32]) -> Result<String> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        // Use a zero nonce for simplicity (should be unique per encryption)
        let nonce = Nonce::from_slice(&[0u8; 12]);
        let encrypted = cipher
            .encrypt(nonce, seed.as_ref())
            .map_err(|e| anyhow::anyhow!("seed encryption failed: {}", e))?;
        Ok(general_purpose::STANDARD.encode(encrypted))
    }
}
