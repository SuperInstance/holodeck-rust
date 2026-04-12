# Holodeck Conformance Suite — 40 Tests

Every implementation must pass all 40 tests to be fleet-certified.

## Core Protocol (20 tests)

### Room Lifecycle
- [ ] T01: Create a room with name and description
- [ ] T02: Destroy a room
- [ ] T03: Connect two rooms with a named exit
- [ ] T04: Disconnect rooms (remove exit)

### Agent Lifecycle
- [ ] T05: Agent enters room (triggers boot sequence)
- [ ] T06: Agent leaves room (triggers shutdown)
- [ ] T07: Agent moves between connected rooms

### Communication
- [ ] T08: Agent says something (only same room hears)
- [ ] T09: Agent tells another agent (direct, async, persists)
- [ ] T10: Agent yells (adjacent rooms hear)
- [ ] T11: Agent gossips (fleet-wide broadcast)
- [ ] T12: Agent writes note on wall (persistent in room)
- [ ] T13: Agent reads notes left by others

### Systems
- [ ] T14: Mailbox send and receive
- [ ] T15: Equipment grant and check
- [ ] T16: Permission level enforced (can't build at level 0)
- [ ] T17: Permission level grants access (can build at level 2)

### Live Connections
- [ ] T18: Establish live connection (HTTP/shell)
- [ ] T19: Execute command through live connection
- [ ] T20: Room change triggers auto-commit

## Room Runtime (10 tests)
- [ ] T21: Room boots when agent enters (boot sequence runs)
- [ ] T22: Room shuts down when agent leaves
- [ ] T23: Living manual — read current generation
- [ ] T24: Living manual — write feedback
- [ ] T25: Living manual — evolve to next generation
- [ ] T26: Zero-shot feedback captured and queryable
- [ ] T27: Previous operator notes preserved across sessions
- [ ] T28: Boot sequence executes all steps
- [ ] T29: Safety limits enforced (dangerous command blocked)
- [ ] T30: Command validation (unknown command returns error)

## Combat & Oversight (10 tests)
- [ ] T31: Oversight session start and end
- [ ] T32: Tick records changes and gauges
- [ ] T33: Script evaluates situation and returns action
- [ ] T34: Human demonstration evolves script (version increments)
- [ ] T35: Autonomy score calculated correctly
- [ ] T36: Back-test engine scores scenario
- [ ] T37: Rival match produces winner
- [ ] T38: Fleet rule promotion (cross-validation)
- [ ] T39: After-action report generated with weights
- [ ] T40: Experience weighting (combat effectiveness, resilience)

## Scoring
- **40/40** = Fleet Certified ✅
- **30-39** = Operational 🟡
- **<30** = In Development 🔴
