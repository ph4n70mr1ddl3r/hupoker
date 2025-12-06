use server::audit_log::AuditLog;
use game_engine::{HandId, TableId};
use uuid::Uuid;
use base64::Engine;
use std::fs;

#[test]
fn test_encrypt_decrypt_seed() {
    let key = [0x42u8; 32];
    let seed = [0x99u8; 32];
    
    // Encrypt using zero nonce (default encrypt_seed)
    let encrypted = AuditLog::encrypt_seed(seed, &key).unwrap();
    // decrypt_seed expects nonce base64 (zero nonce)
    let zero_nonce = base64::engine::general_purpose::STANDARD.encode([0u8; 12]);
    let decrypted = AuditLog::decrypt_seed(&encrypted, &zero_nonce, &key).unwrap();
    assert_eq!(decrypted, seed);
}

#[test]
fn test_audit_log_without_encryption() {
    let temp_path = "test_audit_no_encryption.log";
    let mut audit_log = AuditLog::new(temp_path, None).unwrap();
    let hand_id = HandId::new(Uuid::new_v4());
    let table_id = TableId::new("test-table".to_string());
    let seed = [0xAAu8; 32];
    audit_log.log_seed(hand_id, &table_id, &seed).unwrap();
    audit_log.log_action(hand_id, 0, game_engine::ActionKind::Check, None).unwrap();
    audit_log.log_hand_end(hand_id, &table_id, vec![0], 1000).unwrap();
    // read file and verify it contains JSON lines
    let content = fs::read_to_string(temp_path).unwrap();
    assert!(content.contains("HandStart"));
    assert!(content.contains("Action"));
    assert!(content.contains("HandEnd"));
    fs::remove_file(temp_path).unwrap();
}

#[test]
fn test_audit_log_with_encryption() {
    let key = [0x55u8; 32];
    let temp_path = "test_audit_with_encryption.log";
    let mut audit_log = AuditLog::new(temp_path, Some(key)).unwrap();
    let hand_id = HandId::new(Uuid::new_v4());
    let table_id = TableId::new("test-table".to_string());
    let seed = [0xBBu8; 32];
    audit_log.log_seed(hand_id, &table_id, &seed).unwrap();
    audit_log.log_action(hand_id, 1, game_engine::ActionKind::Bet, Some(500)).unwrap();
    audit_log.log_hand_end(hand_id, &table_id, vec![1], 2000).unwrap();
    let content = fs::read_to_string(temp_path).unwrap();
    assert!(content.contains("HandStart"));
    assert!(content.contains("rng_seed_encrypted"));
    assert!(content.contains("nonce"));
    fs::remove_file(temp_path).unwrap();
}