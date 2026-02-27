use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use chrono::{DateTime, Utc};
use game_engine::{ActionKind, HandId, TableId};
use getrandom::getrandom;
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

/// Audit log that records all hand events.
/// When encryption is enabled via a cipher, RNG seeds are encrypted before logging
/// to ensure cryptographic integrity. Other events are logged as JSON.
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
        nonce: String,              // base64 nonce used for encryption
        timestamp: DateTime<Utc>,
    },
    Action {
        hand_id: HandId,
        seat: u8,
        kind: String,
        amount: Option<u64>,
        timestamp: DateTime<Utc>,
    },
    HandEnd {
        hand_id: HandId,
        table_id: TableId,
        winner_seats: Vec<u8>,
        pot_amount: u64,
        timestamp: DateTime<Utc>,
    },
}

impl AuditLog {
    pub fn new(path: impl AsRef<Path>, encryption_key: Option<[u8; 32]>) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.as_ref())
            .with_context(|| format!("failed to open audit log {}", path.as_ref().display()))?;
        let writer = BufWriter::new(file);
        let cipher = encryption_key.map(|key| {
            let key = Key::from_slice(&key);
            ChaCha20Poly1305::new(key)
        });
        Ok(Self { writer, cipher })
    }

    pub fn log_event(&mut self, event: AuditEvent) -> Result<()> {
        let json = serde_json::to_string(&event).context("failed to serialize audit event")?;
        writeln!(self.writer, "{json}").context("failed to write audit log")?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush().context("failed to flush audit log")
    }

    pub fn log_seed(&mut self, hand_id: HandId, table_id: &TableId, seed: &[u8; 32]) -> Result<()> {
        let cipher = match &self.cipher {
            Some(c) => c,
            None => {
                return Err(anyhow::anyhow!(
                    "cannot log seed without encryption: encryption key must be configured"
                ));
            }
        };
        let mut nonce_bytes = [0u8; 12];
        getrandom(&mut nonce_bytes)
            .map_err(|e| anyhow::anyhow!("failed to generate nonce: {e}"))?;
        let nonce_obj = Nonce::from_slice(&nonce_bytes);
        let encrypted = cipher
            .encrypt(nonce_obj, seed.as_ref())
            .map_err(|e| anyhow::anyhow!("seed encryption failed: {e}"))?;
        let nonce = general_purpose::STANDARD.encode(nonce_bytes);
        let rng_seed_encrypted = general_purpose::STANDARD.encode(encrypted);
        let event = AuditEvent::HandStart {
            hand_id,
            table_id: table_id.clone(),
            rng_seed_encrypted,
            nonce,
            timestamp: Utc::now(),
        };
        self.log_event(event)?;
        self.flush()
    }

    pub fn log_action(
        &mut self,
        hand_id: HandId,
        seat: u8,
        kind: ActionKind,
        amount: Option<u64>,
    ) -> Result<()> {
        let event = AuditEvent::Action {
            hand_id,
            seat,
            kind: kind.to_string(),
            amount,
            timestamp: Utc::now(),
        };
        self.log_event(event)
    }

    pub fn log_hand_end(
        &mut self,
        hand_id: HandId,
        table_id: &TableId,
        winner_seats: Vec<u8>,
        pot_amount: u64,
    ) -> Result<()> {
        let event = AuditEvent::HandEnd {
            hand_id,
            table_id: table_id.clone(),
            winner_seats,
            pot_amount,
            timestamp: Utc::now(),
        };
        self.log_event(event)?;
        self.flush()
    }

    pub fn encrypt_seed(seed: [u8; 32], key: &[u8; 32]) -> Result<String> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let mut nonce_bytes = [0u8; 12];
        getrandom(&mut nonce_bytes)
            .map_err(|e| anyhow::anyhow!("failed to generate nonce: {e}"))?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        let encrypted = cipher
            .encrypt(nonce, seed.as_ref())
            .map_err(|e| anyhow::anyhow!("seed encryption failed: {e}"))?;
        let nonce_b64 = general_purpose::STANDARD.encode(nonce_bytes);
        let encrypted_b64 = general_purpose::STANDARD.encode(encrypted);
        Ok(format!("{nonce_b64}:{encrypted_b64}"))
    }

    pub fn decrypt_seed(encrypted_with_nonce: &str, key: &[u8; 32]) -> Result<[u8; 32]> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        // Check format before splitting to prevent panic on malformed input
        if !encrypted_with_nonce.contains(':') {
            return Err(anyhow::anyhow!("invalid encrypted seed format, expected nonce:encrypted"));
        }
        let parts: Vec<&str> = encrypted_with_nonce.splitn(2, ':').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(anyhow::anyhow!("invalid encrypted seed format, expected nonce:encrypted"));
        }
        let nonce_bytes = general_purpose::STANDARD
            .decode(parts[0])
            .map_err(|e| anyhow::anyhow!("failed to decode nonce: {e}"))?;
        if nonce_bytes.len() != 12 {
            return Err(anyhow::anyhow!("nonce must be 12 bytes, got {}", nonce_bytes.len()));
        }
        let nonce = Nonce::from_slice(&nonce_bytes);
        let encrypted = general_purpose::STANDARD
            .decode(parts[1])
            .map_err(|e| anyhow::anyhow!("failed to decode encrypted seed: {e}"))?;
        let seed = cipher
            .decrypt(nonce, encrypted.as_ref())
            .map_err(|e| anyhow::anyhow!("seed decryption failed: {e}"))?;
        if seed.len() != 32 {
            return Err(anyhow::anyhow!("decrypted seed must be 32 bytes, got {}", seed.len()));
        }
        let mut seed_array = [0u8; 32];
        seed_array.copy_from_slice(&seed);
        Ok(seed_array)
    }
}
