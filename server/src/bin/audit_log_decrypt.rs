use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, Key, KeyInit, Nonce};
use game_engine::{HandId, TableId};
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AuditEvent {
    HandStart {
        hand_id: HandId,
        table_id: TableId,
        rng_seed_encrypted: String,
        nonce: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    Action {
        hand_id: HandId,
        seat: u8,
        kind: String,
        amount: Option<u64>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    HandEnd {
        hand_id: HandId,
        table_id: TableId,
        winner_seats: Vec<u8>,
        pot_amount: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: audit_log_decrypt <audit_log_file>");
        eprintln!(
            "Set HUPOKER_ENCRYPTION_KEY environment variable for seed decryption (hex encoded)."
        );
        std::process::exit(1);
    }
    let path = PathBuf::from(&args[1]);
    let file = File::open(&path).with_context(|| format!("failed to open {:?}", path))?;
    let reader = BufReader::new(file);

    // Optional decryption key
    let key = env::var("HUPOKER_ENCRYPTION_KEY").ok().and_then(|hex_key| {
        hex::decode(hex_key).ok().and_then(|bytes| {
            if bytes.len() == 32 {
                bytes.try_into().ok()
            } else {
                None
            }
        })
    });

    for line in reader.lines() {
        let line = line.with_context(|| "failed to read line")?;
        let event: AuditEvent = serde_json::from_str(&line)
            .with_context(|| format!("failed to parse JSON: {}", line))?;
        match event {
            AuditEvent::HandStart { hand_id, table_id, rng_seed_encrypted, nonce, timestamp } => {
                println!(
                    "[{}] HandStart hand={:?} table={}",
                    timestamp,
                    hand_id,
                    table_id.as_str()
                );
                if let Some(key) = key.as_ref() {
                    if !nonce.is_empty() {
                        match decrypt_seed(&rng_seed_encrypted, &nonce, key) {
                            Ok(seed) => println!("   Decrypted seed: {}", hex::encode(seed)),
                            Err(e) => println!("   Decryption failed: {}", e),
                        }
                    } else {
                        // No encryption, seed is plain base64
                        match general_purpose::STANDARD.decode(&rng_seed_encrypted) {
                            Ok(seed) => println!("   Plain seed: {}", hex::encode(seed)),
                            Err(e) => println!("   Failed to decode seed: {}", e),
                        }
                    }
                }
            }
            AuditEvent::Action { hand_id, seat, kind, amount, timestamp } => {
                println!(
                    "[{}] Action hand={:?} seat={} kind={} amount={:?}",
                    timestamp, hand_id, seat, kind, amount
                );
            }
            AuditEvent::HandEnd { hand_id, table_id, winner_seats, pot_amount, timestamp } => {
                println!(
                    "[{}] HandEnd hand={:?} table={} winners={:?} pot={}",
                    timestamp,
                    hand_id,
                    table_id.as_str(),
                    winner_seats,
                    pot_amount
                );
            }
        }
    }
    Ok(())
}

fn decrypt_seed(encrypted_b64: &str, nonce_b64: &str, key: &[u8; 32]) -> Result<[u8; 32]> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce_bytes =
        general_purpose::STANDARD.decode(nonce_b64).with_context(|| "failed to decode nonce")?;
    if nonce_bytes.len() != 12 {
        anyhow::bail!("nonce must be 12 bytes");
    }
    let nonce = Nonce::from_slice(&nonce_bytes);
    let encrypted = general_purpose::STANDARD
        .decode(encrypted_b64)
        .with_context(|| "failed to decode encrypted seed")?;
    let seed = cipher
        .decrypt(nonce, encrypted.as_ref())
        .map_err(|e| anyhow::anyhow!("seed decryption failed: {:?}", e))?;
    if seed.len() != 32 {
        anyhow::bail!("decrypted seed must be 32 bytes");
    }
    let mut seed_array = [0u8; 32];
    seed_array.copy_from_slice(&seed);
    Ok(seed_array)
}
