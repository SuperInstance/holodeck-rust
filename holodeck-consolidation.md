# Holodeck Consolidation Audit

## Summary

**holodeck-rust** (v0.3.2) and **holodeck-core** (v0.1.0) share identical source files for the 8 overlapping modules — room, agent, comms, combat, gauge, permission, manual, npc. The sole difference is `uuid::Uuid::new_v4()` vs `SystemTime::now()` for ID generation.

**holodeck-core is not actually a no_std crate.** Both have identical dependencies (tokio, serde, serde_json, chrono). holodeck-rust adds `uuid` and `reqwest`.

holodeck-rust is the canonical repo. holodeck-core appears to be an abandoned attempt to publish on crates.io, but it's stale — same code, older version.

## Crates.io Status

| Crate | Max Version | Published |
|-------|-------------|-----------|
| holodeck-rust | 0.3.2 | ✅ |
| holodeck-core | 0.1.0 | ✅ |

## Code Overlap

- **8 identical modules**: room, agent, comms, combat, gauge, permission, manual, npc
- **holodeck-rust only**: holodeck (program runner), director, evolution, games, npc_refresh, plato_bridge, sentiment_npc, sonar_vision, main.rs binary
- **holodeck-core only**: stub comments indicating full versions live elsewhere (holodeck-combat, holodeck-programs, holodeck-bridge)

## Recommendation: **Merge — keep holodeck-rust, deprecate holodeck-core**

1. **holodeck-core is not used** — no imports of `holodeck_core` found in holodeck-rust
2. **Same code, no std isolation** — holodeck-core has no std exclusion, identical deps
3. **Stale version** — holodeck-rust (0.3.2) is 3x version ahead of holodeck-core (0.1.0)
4. **Confusing duplication** — devs may wonder which to use
5. **Stub comments in holodeck-core** say full impl is in other crates — it's not a clean extraction

### If keeping separate crates:
- holodeck-core should `cargo publish --dry-run` verify no actual extraction happened
- Version 0.1.0 is already out, so yank from crates.io or bump to 0.2.0 with a clear purpose

### If merging:
- holodeck-rust remains the single crate
- holodeck-core: `cargo owner add superinstance` then yank 0.1.0 or deprecate

## Action Items

- [ ] Yank holodeck-core@0.1.0 from crates.io (or confirm it can be unpublished)
- [ ] Add deprecation note to holodeck-core README pointing to holodeck-rust
- [ ] Optionally: publish holodeck-rust with full feature flags for the "core" subset