# Holodeck Rust — Success & Failure Log

## v0.3 Build Log (2026-04-13)

### Successes ✅

1. **Zero unsafe Rust** — Destructuring structs gives independent mutable borrows. No raw pointers needed.
2. **8 modules, all compose cleanly** — gauge/combat/comms/manual/permission/room/agent/npc
3. **Full playtest passed** — 20+ commands, all working. Real gauge updates, threshold alerts, combat ticks.
4. **NPC system with static greetings** — 5 NPCs in 5 rooms, immediate response, zero latency.
5. **Seed-2.0-mini quest decomposition** — $0.0008 for detailed 5-step quests. Inbetweener pattern works.
6. **NPC dialogue refresh generates context-aware greetings** — NPCs reference live gauge data.
7. **Permission model** — 6 levels with granular access. Crew can do most things, Architect can modify ship.

### Failures ❌

1. **E0499 double mutable borrows** — First attempt used `&mut self.agents` while borrowing `&mut self.agents` in the same method. Fix: remove agent from HashMap, mutate standalone, reinsert.
2. **E0499 with ShipState fields** — `s.agents` and `s.rooms` both under same `RwLock<ShipState>`. Fix: destructure `let ShipState { rooms, agents, .. } = &mut *s;` to get independent borrows.
3. **unsafe pointer hack** — Tried `std::ptr::addr_of_mut!` to bypass borrow checker. Worked but ugly. Replaced with struct destructuring. Lesson: never fight the borrow checker, restructure.
4. **Float comparison in tests** — `assert_eq!(0.85f32, json_value)` fails due to f32 precision. Fix: `assert!((a - b).abs() < 0.01)`.
5. **HashMap::values() doesn't return (K,V)** — Used `.values().map(|(k,v)|)` which is `.iter()` pattern. Fix: `.iter()` for (K,V), `.values()` for V only.
6. **sync curl blocks async server** — `std::process::Command` in tokio async context freezes ALL connections for 15+ seconds. Fix attempt: `spawn_blocking` works but response delivery is broken.
7. **refreshnpcs response delivery** — spawn_blocking completes but response goes to dead nc connection. Cap session gets stuck. Real fix needed: async HTTP (reqwest) + response queue.
8. **Model name case sensitivity** — DeepInfra requires `ByteDance/Seed-2.0-mini` (capital B). JC1 had `bytedance/Seed-2.0-mini`. Silent failure until debugging.
9. **Timeout handling** — Seed-2.0-mini sometimes returns empty (timeout after 15s). Need retry logic.
10. **Rust compilation caching** — First build took 22s, subsequent 2-3s. Always build release for production.

### Lessons

- **The borrow checker teaches architecture.** Every E0499 was a signal that the code structure was wrong, not that Rust was being annoying.
- **Static + dynamic hybrid for NPCs.** Static greetings for instant response, async refresh for context-aware updates. Never block for creativity.
- **Test the failure paths.** Every success was obvious. Every failure was a surprise that taught something.
- **Document while building.** These notes are more valuable than the code — the code can be rewritten, the learning can't.
