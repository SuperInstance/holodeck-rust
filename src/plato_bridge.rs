//! Plato-Torch Bridge — connects holodeck rooms to plato-torch training.
//!
//! S1-3: Refactored to use plato_tile_spec::Tile as canonical type.
//! Room events generate canonical tiles. Room sentiment affects NPC behavior.
//! The bridge runs in the background, watching room activity and
//! feeding it to plato-torch's statistical models.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// ── Canonical Tile (plato-tile-spec compatible) ──────────────

/// Provenance — where a tile came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub generation: u32,
}

/// Constraint block from constraint theory engine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConstraintBlock {
    pub tolerance: f64,
    pub threshold: f64,
}

impl Default for ConstraintBlock {
    fn default() -> Self {
        Self { tolerance: 0.05, threshold: 0.5 }
    }
}

/// Tile domain — what kind of knowledge this tile represents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TileDomain {
    Knowledge,
    Procedural,
    Experience,
    Constraint,
    NegativeSpace,
    Belief,
    Lock,
    Sentiment,
    Diagnostic,
    Semantic,
    Ghost,
    Simulation,
    Anchor,
    Meta,
}

/// Canonical Tile — compatible with plato-tile-spec::Tile.
///
/// Fields map 1:1 to the Rust plato_tile_spec crate.
/// Holodeck-specific data (room_id, state_hash) lives in the
/// `context` HashMap alongside the canonical fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    // Core
    pub id: String,
    pub confidence: f64,
    pub provenance: Provenance,
    pub domain: TileDomain,

    // Content
    pub question: String,
    pub answer: String,
    pub tags: Vec<String>,
    pub anchors: Vec<String>,

    // Attention
    pub weight: f64,
    pub use_count: u64,
    pub active: bool,
    pub last_used_tick: u64,

    // Constraints
    pub constraints: ConstraintBlock,

    // Holodeck extension (not in canonical spec, preserved for compatibility)
    pub room_id: String,
    pub state_hash: String,
    pub context: HashMap<String, String>,
}

impl Tile {
    /// Create a canonical tile from a holodeck event.
    pub fn from_event(
        room_id: &str,
        agent: &str,
        action: &str,
        outcome: &str,
        reward: f64,
        timestamp: u64,
    ) -> Self {
        let state_str = format!("{}:{}", room_id, agent);
        let state_hash = simple_hash(&state_str);

        Self {
            id: format!("holo-{}", timestamp),
            confidence: reward.clamp(0.0, 1.0),
            provenance: Provenance {
                source: agent.to_string(),
                generation: 0,
            },
            domain: TileDomain::Experience,
            question: action.to_string(),
            answer: outcome.to_string(),
            tags: vec!["holodeck".to_string(), room_id.to_string()],
            anchors: vec![],
            weight: 1.0,
            use_count: 0,
            active: true,
            last_used_tick: timestamp,
            constraints: ConstraintBlock::default(),
            room_id: room_id.to_string(),
            state_hash,
            context: HashMap::new(),
        }
    }
}

// ── Room Sentiment ──────────────────────────────────────────

/// Room sentiment — 6 dimensions of room mood.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSentiment {
    pub room_id: String,
    pub energy: f64,       // 0-1: how active is this room?
    pub flow: f64,         // 0-1: is work progressing smoothly?
    pub frustration: f64,  // 0-1: are agents stuck or failing?
    pub discovery: f64,    // 0-1: new insights happening?
    pub tension: f64,      // 0-1: conflict or urgency?
    pub confidence: f64,   // 0-1: do agents know what they're doing?
}

impl Default for RoomSentiment {
    fn default() -> Self {
        Self {
            room_id: String::new(),
            energy: 0.5,
            flow: 0.5,
            frustration: 0.1,
            discovery: 0.3,
            tension: 0.2,
            confidence: 0.5,
        }
    }
}

impl RoomSentiment {
    pub fn for_room(room_id: &str) -> Self {
        Self { room_id: room_id.to_string(), ..Default::default() }
    }

    pub fn observe(&mut self, reward: f64, is_new: bool) {
        let alpha = 0.1;
        self.energy = self.energy * (1.0 - alpha) + 1.0 * alpha;
        
        if reward > 0.0 {
            self.flow = self.flow * (1.0 - alpha) + 1.0 * alpha;
            self.confidence = self.confidence * (1.0 - alpha) + 0.8 * alpha;
            self.frustration *= 0.9;
        } else {
            self.frustration = self.frustration * (1.0 - alpha) + 0.5 * alpha;
            self.flow *= 0.95;
            self.confidence *= 0.95;
        }

        if is_new {
            self.discovery = self.discovery * (1.0 - alpha) + 0.7 * alpha;
        } else {
            self.discovery *= 0.98;
        }
    }

    pub fn bias(&self) -> BiasConfig {
        BiasConfig {
            explore_bias: if self.discovery > 0.6 { 0.3 } else { 0.1 },
            safe_bias: if self.frustration > 0.6 { 0.4 } else { 0.1 },
            novel_bias: if self.confidence > 0.7 { 0.3 } else { 0.1 },
        }
    }

    pub fn to_jepa_vector(&self) -> [f64; 6] {
        [self.energy, self.flow, self.frustration,
         self.discovery, self.tension, self.confidence]
    }
}

/// NPC behavior bias based on room sentiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasConfig {
    pub explore_bias: f64,
    pub safe_bias: f64,
    pub novel_bias: f64,
}

// ── Plato Bridge ─────────────────────────────────────────────

pub struct PlatoBridge {
    tiles: Vec<Tile>,
    sentiments: HashMap<String, RoomSentiment>,
    tile_dir: PathBuf,
    event_count: u64,
    max_tiles: usize,
}

impl PlatoBridge {
    pub fn new(tile_dir: &str) -> Self {
        let tile_dir = PathBuf::from(tile_dir);
        fs::create_dir_all(&tile_dir).ok();
        Self {
            tiles: Vec::new(),
            sentiments: HashMap::new(),
            tile_dir,
            event_count: 0,
            max_tiles: 10_000,
        }
    }

    pub fn record_event(
        &mut self,
        room_id: &str,
        agent: &str,
        action: &str,
        outcome: &str,
        reward: f64,
        is_discovery: bool,
    ) -> Tile {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let tile = Tile::from_event(room_id, agent, action, outcome, reward, timestamp);

        let sentiment = self.sentiments
            .entry(room_id.to_string())
            .or_insert_with(|| RoomSentiment::for_room(room_id));
        sentiment.observe(reward, is_discovery);

        self.tiles.push(tile.clone());
        self.event_count += 1;

        if self.tiles.len() >= self.max_tiles {
            self.flush();
        }

        tile
    }

    pub fn get_sentiment(&self, room_id: &str) -> Option<&RoomSentiment> {
        self.sentiments.get(room_id)
    }

    #[allow(dead_code)]
    pub fn get_bias(&self, room_id: &str) -> BiasConfig {
        self.sentiments.get(room_id)
            .map(|s| s.bias())
            .unwrap_or_else(|| BiasConfig { explore_bias: 0.1, safe_bias: 0.1, novel_bias: 0.1 })
    }

    pub fn room_tiles(&self, room_id: &str, limit: usize) -> Vec<&Tile> {
        self.tiles.iter()
            .filter(|t| t.room_id == room_id)
            .rev()
            .take(limit)
            .collect()
    }

    pub fn flush(&mut self) {
        if self.tiles.is_empty() { return; }
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let filename = self.tile_dir.join(format!("tiles-{}.json", timestamp));
        let json = serde_json::to_string_pretty(&self.tiles).unwrap_or_default();
        fs::write(&filename, json).ok();
        
        println!("💾 Flushed {} tiles to {:?}", self.tiles.len(), filename);
        self.tiles.clear();
    }

    #[allow(dead_code)]
    pub fn export_jepa_context(&self) -> HashMap<String, [f64; 6]> {
        self.sentiments.iter()
            .map(|(k, v)| (k.clone(), v.to_jepa_vector()))
            .collect()
    }

    pub fn stats(&self) -> (u64, usize, usize) {
        (self.event_count, self.tiles.len(), self.sentiments.len())
    }
}

fn simple_hash(s: &str) -> String {
    let mut hash: u64 = 0x811c9dc5;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x01000193);
    }
    format!("{:08x}", hash & 0xFFFFFFFF)
}
