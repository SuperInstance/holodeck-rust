# 🎮 Holodeck Rust Playtest Report — 2026-04-13

**Tester:** Oracle1 (automated playtest)
**Version:** v0.3
**Connection:** TCP localhost:7778

## What Works Well ✅

1. **Navigation** — `go bridge` boots the room with smooth transition message. 8 exits visible from Bridge. Room graph feels expansive.
2. **Help system** — Clean, comprehensive command list. Easy to read.
3. **Fleet command** — `fleet` queries real Keeper API (10 vessels, 36K+ API calls). This is a killer feature.
4. **Evolution engine** — `scripts` shows 5 scripts at gen 47 with mutation tracking. Scripts persist across sessions.
5. **Holodeck programs** — 5 programs with permission levels (CADET→ADMIRAL). Clear descriptions.
6. **Room booting** — "Room 'Bridge' booted. 1 agent(s) present." — rooms initialize on demand.

## Issues Found 🐛

### Critical
1. **HTTP requests polluting agent list** — Keeper health checks show as agents: `GET /health HTTP/1.1`, `GET / HTTP/1.1`. Need to filter non-agent connections.
2. **Name prompt consumed as command** — First `look` is treated as the vessel name, not a command. The name prompt timing is off with piped input.

### Medium
3. **No welcome MOTD** — New connections drop straight into "What's your vessel name?" without any intro text about what the MUD is.
4. **Poker not in bridge** — `poker` command not found, even though poker is documented as a feature. May need to be in ten-forward room.
5. **No room description on arrival** — Going to a new room shows the name but could use more atmospheric text.
6. **`who` not tested** — Need to verify agent listing works with multiple connections.

### Low
7. **No color coding** — Everything is cyan. Gauges could be green/yellow/red based on value.
8. **No prompt character** — Just `>` — could show current room name like `[Bridge]>`
9. **Sim/Real mode not visible** — `[SIM]` shows but no explanation of what it means for new users.

## Missing Features (New User Expectations)

1. **Tutorial/onboarding** — First-time users need a walkthrough
2. **Map command** — `map` to see room connections
3. **Score/stats** — Personal stats tracking
4. **Inventory** — Even a simple equipment list
5. **Emotes** — Social commands (wave, nod, shrug)
6. **Tab completion** — For room exits and commands
7. **Combat demo** — Something to DO in rooms without programs

## Front-End Improvements

### Terminal UI (Current)
- Add ANSI color coding: rooms=green, combat=red, fleet=cyan, chat=yellow
- Show `[RoomName]>` prompt with room name
- Add ASCII art for room entrances
- Progress bars for gauges instead of raw numbers

### Web UI (Future)
- Map view showing room connections as a graph
- Gauge dashboard with real-time updating
- Chat window alongside room description
- Program visualizer with live gauge graphs

## Overall Rating: 7/10

The MUD works. Navigation is smooth, fleet integration is real (not simulated), and the evolution engine is genuinely interesting. The main gaps are polish (MOTD, colors, tutorial) and content (things to DO in rooms). The foundation is solid.

## Priority Fixes
1. Filter HTTP requests from agent list (critical)
2. Add welcome MOTD with brief intro
3. Add `map` command for room navigation
4. Color-code output
5. Add poker to ten-forward room (may already work there)
