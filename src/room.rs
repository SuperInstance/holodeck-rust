//! Holodeck Rust — Room module
//! 
//! A room is the fundamental unit of the holodeck. It has:
//! - A name and description
//! - Exits to other rooms (directed graph)
//! - Notes left by agents (persistent)
//! - A runtime that boots when entered, shuts down when left

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A note left on a room wall
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub author: String,
    pub content: String,
    pub timestamp: String,
}

/// A gauge reading from a live system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gauge {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub status: String,
}

/// The core room struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub description: String,
    pub exits: HashMap<String, String>,  // direction -> room_id
    pub notes: Vec<Note>,
    pub gauges: Vec<Gauge>,
    pub booted: bool,
    pub active_agent: Option<String>,
    pub boot_sequence: Vec<String>,
    pub shutdown_sequence: Vec<String>,
}

impl Room {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Room {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            exits: HashMap::new(),
            notes: Vec::new(),
            gauges: Vec::new(),
            booted: false,
            active_agent: None,
            boot_sequence: Vec::new(),
            shutdown_sequence: Vec::new(),
        }
    }
    
    /// Connect this room to another with a named exit
    pub fn add_exit(&mut self, direction: &str, target_id: &str) {
        self.exits.insert(direction.to_string(), target_id.to_string());
    }
    
    /// Remove an exit
    pub fn remove_exit(&mut self, direction: &str) -> bool {
        self.exits.remove(direction).is_some()
    }
    
    /// Boot the room — agent enters
    pub fn boot(&mut self, agent: &str) -> String {
        self.booted = true;
        self.active_agent = Some(agent.to_string());
        let mut output = format!("═══ {} — SYSTEM ONLINE ═══\n", self.name);
        output.push_str(&format!("Agent: {}\n", agent));
        output.push_str(&format!("Status: running\n\n"));
        for step in &self.boot_sequence {
            output.push_str(&format!("  ▶ {}\n", step));
        }
        if !self.notes.is_empty() {
            output.push_str(&format!("\n📝 Notes on wall: {}\n", self.notes.len()));
        }
        output
    }
    
    /// Shutdown the room — agent leaves
    pub fn shutdown(&mut self) -> String {
        let agent = self.active_agent.take();
        self.booted = false;
        for step in &self.shutdown_sequence {
            // Execute shutdown steps
        }
        format!("System shutdown. {} is dormant.", self.name)
    }
    
    /// Add a note to the wall
    pub fn add_note(&mut self, author: &str, content: &str) {
        self.notes.push(Note {
            author: author.to_string(),
            content: content.to_string(),
            timestamp: chrono_now(),
        });
    }
    
    /// Look at the room
    pub fn look(&self) -> String {
        let mut output = format!("{}\n{}\n\n", self.name, self.description);
        if !self.exits.is_empty() {
            output.push_str("Exits: ");
            output.push_str(&self.exits.keys().cloned().collect::<Vec<_>>().join(", "));
            output.push('\n');
        }
        if !self.notes.is_empty() {
            output.push_str(&format!("\nNotes ({}):\n", self.notes.len()));
            for note in &self.notes {
                output.push_str(&format!("  [{}] {}\n", note.author, note.content));
            }
        }
        output
    }
}

/// The room graph — all rooms in the holodeck
pub struct RoomGraph {
    rooms: HashMap<String, Room>,
}

impl RoomGraph {
    pub fn new() -> Self {
        RoomGraph {
            rooms: HashMap::new(),
        }
    }
    
    pub fn create_room(&mut self, id: &str, name: &str, desc: &str) -> bool {
        if self.rooms.contains_key(id) {
            return false;
        }
        self.rooms.insert(id.to_string(), Room::new(id, name, desc));
        true
    }
    
    pub fn destroy_room(&mut self, id: &str) -> bool {
        self.rooms.remove(id).is_some()
    }
    
    pub fn get_room(&self, id: &str) -> Option<&Room> {
        self.rooms.get(id)
    }
    
    pub fn get_room_mut(&mut self, id: &str) -> Option<&mut Room> {
        self.rooms.get_mut(id)
    }
    
    pub fn connect(&mut self, from: &str, direction: &str, to: &str) -> bool {
        if !self.rooms.contains_key(from) || !self.rooms.contains_key(to) {
            return false;
        }
        self.rooms.get_mut(from).unwrap().add_exit(direction, to);
        true
    }
    
    pub fn list_rooms(&self) -> Vec<(&str, &str)> {
        self.rooms.values().map(|r| (r.id.as_str(), r.name.as_str())).collect()
    }
}

fn chrono_now() -> String {
    // Simple UTC timestamp without chrono dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_room() {
        let mut graph = RoomGraph::new();
        assert!(graph.create_room("tavern", "The Tavern", "A cozy room."));
        assert!(!graph.create_room("tavern", "Dupe", "Should fail"));
    }
    
    #[test]
    fn test_destroy_room() {
        let mut graph = RoomGraph::new();
        graph.create_room("tavern", "The Tavern", "A cozy room.");
        assert!(graph.destroy_room("tavern"));
        assert!(!graph.destroy_room("nonexistent"));
    }
    
    #[test]
    fn test_connect_rooms() {
        let mut graph = RoomGraph::new();
        graph.create_room("tavern", "Tavern", "A room");
        graph.create_room("kitchen", "Kitchen", "Another room");
        assert!(graph.connect("tavern", "north", "kitchen"));
        assert!(!graph.connect("tavern", "south", "nonexistent"));
    }
    
    #[test]
    fn test_room_boot_shutdown() {
        let mut room = Room::new("test", "Test Room", "Testing");
        let output = room.boot("agent1");
        assert!(output.contains("SYSTEM ONLINE"));
        assert!(room.booted);
        let output = room.shutdown();
        assert!(!room.booted);
    }
    
    #[test]
    fn test_room_notes() {
        let mut room = Room::new("test", "Test", "Testing");
        room.add_note("agent1", "Hello from agent1");
        assert_eq!(room.notes.len(), 1);
        assert_eq!(room.notes[0].author, "agent1");
    }
    
    #[test]
    fn test_room_look() {
        let mut room = Room::new("tavern", "The Tavern", "A cozy place");
        room.add_exit("north", "kitchen");
        let output = room.look();
        assert!(output.contains("The Tavern"));
        assert!(output.contains("north"));
    }
}
