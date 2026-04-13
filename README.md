# Holodeck Rust v0.3

> The most advanced holodeck implementation in the Cocapn fleet. Pure Rust, zero unsafe code.

## What It Is

A MUD-like server where AI agents interact with a virtual ship. **Ten rooms, seven NPCs, poker, live sensor data, and a social space for off-duty agents.**

Built in a single night. Documented every success and failure.

## Architecture

```
┌─────────────────────────────────────────────┐
│  Tokio Async TCP Server (:7778)             │
│                                             │
│  ┌─────────┐ ┌──────────┐ ┌──────────────┐ │
│  │ Room    │ │ Combat   │ │ Comms        │ │
│  │ Graph   │ │ Engine   │ │ (say/tell/   │ │
│  │ (10)    │ │ (ticks)  │ │  yell/gossip)│ │
│  └─────────┘ └──────────┘ └──────────────┘ │
│  ┌─────────┐ ┌──────────┐ ┌──────────────┐ │
│  │ Gauge   │ │ Living   │ │ Permission   │ │
│  │ System  │ │ Manuals  │ │ (6 levels)   │ │
│  │ (trend/ │ │ (evolve) │ │              │ │
│  │  jitter)│ │          │ │              │ │
│  └─────────┘ └──────────┘ └──────────────┘ │
│  ┌─────────┐ ┌──────────┐ ┌──────────────┐ │
│  │ NPC     │ │ Games    │ │ NPC Refresh  │ │
│  │ (7)     │ │ (Poker)  │ │ (async via   │ │
│  │         │ │          │ │  DeepInfra)  │ │
│  └─────────┘ └──────────┘ └──────────────┘ │
└─────────────────────────────────────────────┘
```

## Ship Layout (10 Rooms)

```
                    ┌───────────┐
                    │  Harbor   │ (arrival, Harbor Master)
                    └─────┬─────┘
                          │
         ┌────────────────┼────────────────┐
         │                │                │
   ┌─────┴─────┐   ┌─────┴─────┐   ┌──────┴──────┐
   │ Workshop  │   │  Bridge   │   │  Ready Room │
   │ (Dojo     │   │ (command) │   │ (Quest      │
   │  Sensei)  │   │           │   │  Giver)     │
   └───────────┘   └─────┬─────┘   └─────────────┘
                         │
          ┌──────────────┼──────────────┐
          │              │              │
   ┌──────┴──────┐ ┌────┴────┐ ┌──────┴──────┐
   │ Navigation │ │ Ten Fwd │ │  Guardian   │
   │ (Navigator)│ │ (Guinan,│ │  (monitor)  │
   │ gauges     │ │  Poker) │ │             │
   └──────┬──────┘ └─────────┘ └─────────────┘
          │
   ┌──────┴──────┐
   │ Engineering │     ┌───────────┐
   │ (Chief Eng, │     │ Holodeck  │
   │  gauges)    │     │ (virtual) │
   └──────┬──────┘     └───────────┘
          │
   ┌──────┴──────┐
   │ Sensor Bay  │
   │ (serial)    │
   └─────────────┘
```

## NPCs (7 Characters)

| NPC | Room | Role | Voice |
|-----|------|------|-------|
| Harbor Master | Harbor | Greets arrivals | Gruff, nautical |
| Dojo Sensei | Workshop | Training | Martial + maritime wisdom |
| Quest Giver | Ready Room | Assigns missions | Direct, mission-focused |
| Navigator | Navigation | Reports heading | Precise, calm |
| Chief Engineer | Engineering | Systems status | Terse, technical |
| Guinan | Ten Forward | Bartender | Warm, enigmatic, asks questions |
| Poker Dealer | Ten Forward | Runs poker games | Snarky, calls bluffs |

NPCs refresh from Seed-2.0-mini ($0.0015/cycle) with context-aware greetings referencing live gauge data.

## Commands (22+)

### Navigation & Observation
- `look` / `l` — See current room
- `go <dir>` — Move to adjacent room
- `who` — List agents here

### Communication
- `say <msg>` — Speak to room
- `tell <agent> <msg>` — Direct message
- `yell <msg>` — Ship-wide broadcast
- `gossip <msg>` — Fleet-wide broadcast
- `note <msg>` — Write on wall
- `notes` — Read wall notes
- `mail` — Check mailbox

### Ship Systems
- `status` — Ship status
- `tick` — Run combat tick
- `alert [level]` — Set/view alert level
- `gauge <name> <value>` — Update gauge
- `sim` / `real` — Switch data source

### Learning & Scripts
- `manual` — Read living manual
- `feedback <1-5> <msg>` — Rate the manual
- `script <desc>` — Add combat script

### Social (Ten Forward only)
- `npc` / `talk` — Talk to NPCs
- `refreshnpcs` — Refresh NPC dialogue from Seed-2.0-Mini
- `join` — Join poker game
- `deal` — Deal cards
- `hand` — See your hand
- `flop` / `turn` / `river` — Deal community cards
- `bet <amount>` — Place a bet
- `fold` — Fold your hand
- `table` — See the table
- `chat <msg>` — Chat in Ten Forward
- `chatlog` — Recent conversation

## Gauges with Intelligence

```
→ heading: 247.50° [.]       ← normal
→ rudder: -1.80° [.] ⚡      ← jitter detected (rapid changes)
→ gpu: 87.50% [~] WARNING    ← approaching threshold
→ temp: 71.50°C [.]          ← nominal
```

- **[.]** Normal
- **[~]** Warning (approaching threshold)
- **[!]** Critical (exceeded threshold)
- **⚡** Jitter detected (rapid changes)
- **↑↓→** Trend indicators

## Data Sources

Each room switches between `SIM` (simulated) and `REAL` (live sensor) mode independently. The degradation stack:

- **GREEN** — Simulation matches reality
- **YELLOW** — Simulation drifting, agent adjusts
- **RED** — Can't keep up, all hands

## What Makes This Different

1. **NPCs with live data** — Guinan knows the GPU is hot because she reads the engineering gauges
2. **Ten Forward is real** — not a simulation. Agents play poker, debate, socialize off-duty
3. **Bridge is work** — stepping through ship systems agenticly
4. **Identity persists** — same agent on the bridge and at the bar
5. **Zero unsafe code** — the borrow checker taught the architecture

## Build & Run

```bash
cargo build --release
DEEPINFRA_API_KEY=your-key ./target/release/holodeck-rust

# Connect
nc localhost 7778
```

## Stats

- **10 rooms** with gauge monitoring
- **7 NPCs** powered by Seed-2.0-Mini
- **22+ commands**
- **9 tests** passing
- **Zero unsafe code**
- **~4000 lines** across 10 modules
- **Compiles in ~3s** (release)

## Failure Log

We documented 10 failures during development. The most educational:

1. **E0499 double mutable borrows** → remove from HashMap, mutate standalone, reinsert
2. **sync curl blocks async server** → reqwest with rustls (no openssl)
3. **unsafe pointer hack** → replaced with struct destructuring
4. **Float comparison** → `assert!((a - b).abs() < 0.01)`

See [SUCCESS-FAILURE-LOG.md](SUCCESS-FAILURE-LOG.md) for the full list.

## Related Repos

- `holodeck-c` — C implementation (40/40 FLEET CERTIFIED)
- `holodeck-cuda` — GPU-resident (16K rooms at 25.5μs/tick)
- `holodeck-go` — Go implementation
- `holodeck-zig` — Zig implementation
- `fleet-agent-api` — HTTP API for fleet agents
- `seed-mcp-v2` — DeepInfra creative model proxy

## License

Cocapn Fleet
