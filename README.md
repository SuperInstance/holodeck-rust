# Holodeck Rust

Safe Rust implementation of the FLUX-LCAR holodeck protocol.

## What It Teaches

What IS ownership of a room? The borrow checker forced us to clone exit targets before mutating rooms. The graph owns rooms, agents borrow them. `Arc<RwLock<RoomGraph>>` for concurrent access. Tokio async: one task per agent, zero-cost abstraction.

## Build

```bash
cargo build    # Build
cargo test     # 11 tests passing
```

## Status

**11/11 unit tests passing** ✅

## Architecture

```
src/
  main.rs   — tokio async runtime, TCP :7778
  room.rs   — room graph with boot/shutdown lifecycle
  agent.rs  — agent sessions, permission levels, command dispatch
```

## Run

```bash
cargo run  # Starts on :7778
```

## The Borrow Checker Lesson

```rust
// Can't mutate room while reading its exits:
let target = room.exits.get(&args).cloned();  // clone FIRST
if let Some(target_id) = target {
    room.shutdown();  // NOW mutate
}
```

Agents borrow rooms. The graph owns them. This IS spatial interaction safety.

## Dependencies

Rust 2021 edition. tokio (async), serde/serde_json (serialization).
