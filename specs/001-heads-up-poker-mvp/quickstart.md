# Quickstart Guide

**Feature**: Heads‑up NLHE Poker MVP (Rust Server + Windows Desktop Client)  
**Date**: 2025‑12‑06  
**Audience**: Developers who want to run, test, or extend the system.

## 1. Prerequisites

- **Rust**: Install the latest stable toolchain via [rustup](https://rustup.rs/).
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
  Verify installation:
  ```bash
  rustc --version   # should be ≥ 1.78
  cargo --version
  ```
- **Git**: The project is version‑controlled with Git. Clone the repository.
- **Windows (for client)**: The desktop client targets Windows 10/11 (64‑bit). Cross‑compilation from Linux is possible but not covered here.
- **Linux (for server)**: The server runs on any Linux distribution (including WSL2). Development on Windows via WSL is fully supported.

## 2. Project Layout

After the implementation tasks are completed, the repository will have the following structure:

```
hupoker/
├── Cargo.toml                    # Workspace definition
├── game_engine/                  # Pure game‑logic crate
│   ├── Cargo.toml
│   └── src/
├── server/                       # Networked server crate
│   ├── Cargo.toml
│   └── src/
├── client_desktop/               # Windows desktop UI crate
│   ├── Cargo.toml
│   └── src/
└── tests/                        # Workspace‑level integration tests
    ├── contract/
    ├── integration/
    └── unit/
```

## 3. Building

From the workspace root (`hupoker/`):

```bash
# Build everything (debug)
cargo build

# Build release binaries (optimized)
cargo build --release
```

Binaries will be placed in `target/debug/` (or `target/release/`):
- `server/hupoker‑server`
- `client_desktop/hupoker‑desktop.exe` (Windows) / `hupoker‑desktop` (Linux)

## 4. Running the Server

1. Create a configuration file `config.toml` (example below).
2. Start the server:
   ```bash
   cd server
   cargo run -- config.toml
   ```
   Or directly:
   ```bash
   ./target/debug/hupoker‑server config.toml
   ```

Example `config.toml`:
```toml
bind_address = "127.0.0.1:8080"
audit_log_path = "/var/log/hupoker/audit.log"
encryption_key_env_var = "HUPOKER_ENCRYPTION_KEY"

[[tables]]
id = "table-1"
small_blind = 10
big_blind = 20
starting_stack = 1500
action_timeout_secs = 30
reconnection_timeout_secs = 60
```

The server will listen on `127.0.0.1:8080` and log to the specified audit path.

**Environment variable**: Set `HUPOKER_ENCRYPTION_KEY` to a base64‑encoded 32‑byte key (for encrypting RNG seeds in the audit log). Example:
```bash
export HUPOKER_ENCRYPTION_KEY="$(openssl rand -base64 32)"
```

## 5. Running the Client (Windows)

1. Ensure the server is running.
2. Launch the client:
   ```bash
   cd client_desktop
   cargo run -- --server 127.0.0.1:8080
   ```
   Or double‑click the built executable.

The client will connect, show available tables, and let you pick a seat.

## 6. Playing a Hand

1. Start the server (as above).
2. Launch **two** client instances (or two separate machines).
3. Each client joins the same table (`table‑1`) on seats `0` and `1`.
4. Once both seats are occupied, the server automatically starts a hand.
5. Follow the on‑screen prompts to act (fold, check, call, bet, raise).
6. The hand proceeds through preflop, flop, turn, river, and showdown.

## 7. Testing

### Unit Tests (Game Engine)
```bash
cargo test -p game_engine
```

### Integration Tests (Full Hand Simulation)
```bash
cargo test -p tests --test full_hand_simulation
```

### Contract Tests (Network Protocol)
```bash
cargo test -p tests --test protocol_tests
```

### All Tests
```bash
cargo test --workspace
```

## 8. Code Quality Checks

```bash
# Formatting
cargo fmt --check

# Linting
cargo clippy --workspace -- -D warnings

# Type checking (already done by `cargo check`)
cargo check --workspace
```

## 9. Development Workflow

1. **Make changes** in the appropriate crate (`game_engine`, `server`, `client_desktop`).
2. **Run tests** to ensure nothing breaks.
3. **Check formatting and linting**.
4. **Commit** with descriptive messages.

## 10. Debugging

- **Server logs**: The server outputs structured logs via `tracing`. Set `RUST_LOG=info` (or `debug`) to see details.
- **Audit log**: The encrypted audit log contains RNG seeds and all game actions. Decrypt with the same key for post‑mortem analysis.
- **Network debugging**: Use `nc` (netcat) or `telnet` to connect to the server port and send raw JSON messages (see `contracts/protocol.md`).

## 11. Common Issues

| Symptom | Likely Cause | Solution |
|---------|--------------|----------|
| Client cannot connect | Server not running / wrong port | Verify server is listening (`netstat -tlnp`). |
| “Seat taken” error | Another client already occupies that seat | Join the other seat, or wait for the other player to disconnect. |
| Action timeout too short | Default is 30 seconds | Increase `action_timeout_secs` in config. |
| Audit log not writable | Permission denied on the log path | Create directory and set appropriate permissions. |
| Missing encryption key | `HUPOKER_ENCRYPTION_KEY` not set | Export the environment variable before starting server. |

## 12. Audit Log Decryption

The server writes an encrypted audit log containing RNG seeds and game actions. To inspect the log, use the `audit_log_decrypt` tool:

```bash
cd server
cargo run --bin audit_log_decrypt -- ../path/to/audit.log
```

Set the `HUPOKER_ENCRYPTION_KEY` environment variable (same key used by the server) to decrypt seeds:

```bash
export HUPOKER_ENCRYPTION_KEY="$(openssl rand -base64 32 | xxd -p -c 32)"
cargo run --bin audit_log_decrypt -- ../path/to/audit.log
```

The tool will print each event with timestamps, and decrypted seeds (if key is provided).

## 13. Next Steps

- Read the **data model** (`data‑model.md`) to understand core entities.
- Study the **network protocol** (`contracts/protocol.md`) for client‑server communication.
- Review the **implementation plan** (`plan.md`) for architectural decisions.
- Explore the **feature specification** (`spec.md`) for user stories and requirements.

Happy coding! 🃏