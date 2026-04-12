# Holodeck Rust — Charter

## Mission
Implement the Holodeck Studio in safe Rust, exploring ownership models for rooms and fearless concurrency.

## Architecture
```
src/
  main.rs         — tokio async runtime, TCP listener
  room.rs         — room graph with Rc/Arc, exits
  agent.rs        — agent session, command dispatch
  command.rs      — command parser and handlers
  comms.rs        — channels (mpsc for gossip, oneshot for tell)
  live.rs         — live connections
  combat.rs       — oversight ticks, evolving scripts
  manual.rs       — living manual
  conformance.rs  — test suite
```

## The Deep Question
What IS ownership of a room? Who owns the room graph? Can an agent "borrow" a room?
What does the borrow checker teach us about spatial interaction safety?
What does fearless concurrency look like when rooms are shared state?
