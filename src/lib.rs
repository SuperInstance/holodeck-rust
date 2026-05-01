//! Holodeck Rust v0.3 — Advanced FLUX-LCAR MUD Library
//!
//! Pure Rust implementation with:
//! - Room graph with gauges, exits, data sources (REAL/SIM)
//! - Scoped communication (say/tell/yell/gossip/note/mail)
//! - Combat engine with evolving scripts
//! - Living manuals that improve across generations
//! - Permission levels (Greenhorn → Architect)

pub mod agent;
pub mod sonar_vision;
pub mod room;
pub mod gauge;
pub mod combat;
pub mod comms;
pub mod manual;
pub mod permission;
pub mod npc;
pub mod npc_refresh;
pub mod games;
pub mod holodeck;
pub mod evolution;
pub mod director;
