//! Plato-Torch Bridge — connects holodeck rooms to plato-torch training.
//!
//! Room events generate tiles. Room sentiment affects NPC behavior.
//! The bridge runs in the background, watching room activity and
//! feeding it to plato-torch's statistical models.

use crate::room::Room;
use crate::gauge::Gauge;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// A tile — the atomic unit of room experience.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub room_id: String,
    pub agent: String,
    pub action: String,
    pub outcome: String,
    pub reward: f64,
    pub timestamp: u64,
    pub state_hash: String,
    pub context: HashMap<String, String>,
}

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

    /// Update sentiment based on a new event.
    pub fn observe(&mut self, reward: f64, is_new: bool) {
        let alpha = 0.1; // EMA smoothing
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

    /// Sentiment affects NPC behavior: biased randomness.
    pub fn bias(&self) -> BiasConfig {
        BiasConfig {
            explore_bias: if self.discovery > 0.6 { 0.3 } else { 0.1 },
            safe_bias: if self.frustration > 0.6 { 0.4 } else { 0.1 },
            novel_bias: if self.confidence > 0.7 { 0.3 } else { 0.1 },
        }
    }

    /// Export as JEPA context vector for edge consumption.
    pub fn to_jepa_vector(&self) -> [f64; 6] {
        [self.energy, self.flow, self.frustration,
         self.discovery, self.tension, self.confidence]
    }
}

/// NPC behavior bias based on room sentiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasConfig {
    pub explore_bias: f64,  // How much to explore vs exploit
    pub safe_bias: f64,     // How much to prefer safe actions
    pub novel_bias: f64,    // How much to try novel approaches
}

/// The bridge between holodeck rooms and plato-torch.
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

    /// Record a room event as a tile.
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

        // State hash from room + agent
        let state_str = format!("{}:{}", room_id, agent);
        let state_hash = simple_hash(&state_str);

        let tile = Tile {
            room_id: room_id.to_string(),
            agent: agent.to_string(),
            action: action.to_string(),
            outcome: outcome.to_string(),
            reward,
            timestamp,
            state_hash,
            context: HashMap::new(),
        };

        // Update room sentiment
        let sentiment = self.sentiments
            .entry(room_id.to_string())
            .or_insert_with(|| RoomSentiment::for_room(room_id));
        sentiment.observe(reward, is_discovery);

        // Store tile
        self.tiles.push(tile.clone());
        self.event_count += 1;

        // Flush to disk if buffer is full
        if self.tiles.len() >= self.max_tiles {
            self.flush();
        }

        tile
    }

    /// Get current sentiment for a room.
    pub fn get_sentiment(&self, room_id: &str) -> Option<&RoomSentiment> {
        self.sentiments.get(room_id)
    }

    /// Get bias config for a room (for NPC behavior).
    pub fn get_bias(&self, room_id: &str) -> BiasConfig {
        self.sentiments.get(room_id)
            .map(|s| s.bias())
            .unwrap_or_else(|| BiasConfig { explore_bias: 0.1, safe_bias: 0.1, novel_bias: 0.1 })
    }

    /// Get recent tiles for a room.
    pub fn room_tiles(&self, room_id: &str, limit: usize) -> Vec<&Tile> {
        self.tiles.iter()
            .filter(|t| t.room_id == room_id)
            .rev()
            .take(limit)
            .collect()
    }

    /// Flush tiles to disk (plato-torch compatible JSON).
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

    /// Export all room sentiments as JEPA context vectors.
    pub fn export_jepa_context(&self) -> HashMap<String, [f64; 6]> {
        self.sentiments.iter()
            .map(|(k, v)| (k.clone(), v.to_jepa_vector()))
            .collect()
    }

    /// Get stats.
    pub fn stats(&self) -> (u64, usize, usize) {
        (self.event_count, self.tiles.len(), self.sentiments.len())
    }
}

fn simple_hash(s: &str) -> String {
    // FNV-1a inspired simple hash (no external deps)
    let mut hash: u64 = 0x811c9dc5;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x01000193);
    }
    format!("{:08x}", hash & 0xFFFFFFFF)
}