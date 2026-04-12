//! Holodeck Rust — Agent module

use crate::room::RoomGraph;
use std::collections::HashMap;

/// Permission levels (matching PLATO model)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionLevel {
    Greenhorn = 0,
    Crew = 1,
    Specialist = 2,
    Captain = 3,
    Cocapn = 4,
    Architect = 5,
}

/// An agent session in the holodeck
#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub room_id: String,
    pub level: PermissionLevel,
    pub mask: Option<String>,
    pub status: String,
}

impl Agent {
    pub fn new(name: &str, start_room: &str) -> Self {
        Agent {
            name: name.to_string(),
            room_id: start_room.to_string(),
            level: PermissionLevel::Greenhorn,
            mask: None,
            status: "active".to_string(),
        }
    }
    
    pub fn can(&self, action: &str) -> bool {
        match action {
            "look" | "go" | "say" | "help" | "who" | "read" => true,
            "tell" | "yell" | "gossip" | "note" | "mail" => self.level >= PermissionLevel::Crew,
            "build" | "summon" | "equip" => self.level >= PermissionLevel::Specialist,
            "adventure" | "baton" | "riff" => self.level >= PermissionLevel::Captain,
            "create_type" | "fleet" => self.level >= PermissionLevel::Cocapn,
            "all" => self.level >= PermissionLevel::Architect,
            _ => false,
        }
    }
    
    pub fn display_name(&self) -> &str {
        self.mask.as_deref().unwrap_or(&self.name)
    }
    
    /// Process a command and return (response, should_quit)
    pub fn handle_command(&mut self, input: &str, rooms: &mut RoomGraph, 
                          agents: &mut HashMap<String, Agent>) -> (String, bool) {
        let parts: Vec<&str> = input.trim().splitn(2, ' ').collect();
        let cmd = parts.get(0).unwrap_or(&"").to_lowercase();
        let args = parts.get(1).unwrap_or(&"").to_string();
        
        match cmd.as_str() {
            "look" | "l" => {
                if let Some(room) = rooms.get_room(&self.room_id) {
                    (room.look(), false)
                } else {
                    ("You are nowhere.".to_string(), false)
                }
            }
            "go" | "move" => {
                if args.is_empty() {
                    ("Go where? Specify a direction.".to_string(), false)
                } else {
                    // Clone the exit target to avoid borrow issues
                    let target_opt = rooms.get_room(&self.room_id)
                        .and_then(|r| r.exits.get(&args).cloned());
                    if let Some(target_id) = target_opt {
                        // Shutdown old room
                        if let Some(old) = rooms.get_room_mut(&self.room_id) {
                            old.shutdown();
                        }
                        self.room_id = target_id.clone();
                        // Boot new room
                        let output = if let Some(new_room) = rooms.get_room_mut(&self.room_id) {
                            new_room.boot(&self.name)
                        } else {
                            "Room not found.".to_string()
                        };
                        (output, false)
                    } else {
                        (format!("No exit '{}' here.", args), false)
                    }
                }
            }
            "say" => {
                if args.is_empty() {
                    ("Say what?".to_string(), false)
                } else {
                    (format!("You say: {}", args), false)
                }
            }
            "tell" => {
                if args.is_empty() {
                    ("Tell whom what?".to_string(), false)
                } else {
                    let tell_parts: Vec<&str> = args.splitn(2, ' ').collect();
                    let target = tell_parts.get(0).unwrap_or(&"");
                    let msg = tell_parts.get(1).unwrap_or(&"");
                    (format!("You tell {}: {}", target, msg), false)
                }
            }
            "who" => {
                let mut output = "═══ Fleet Roster ═══\n".to_string();
                for (name, agent) in agents.iter() {
                    let room = rooms.get_room(&agent.room_id)
                        .map(|r| r.name.as_str())
                        .unwrap_or("nowhere");
                    output.push_str(&format!("  {} — {}\n", name, room));
                }
                (output, false)
            }
            "help" => {
                ("Commands: look, go <dir>, say <msg>, tell <agent> <msg>, who, help, quit\n".to_string(), false)
            }
            "quit" | "exit" => {
                if let Some(room) = rooms.get_room_mut(&self.room_id) {
                    room.shutdown();
                }
                ("Fair winds. The fleet will remember you.".to_string(), true)
            }
            _ => (format!("Unknown command: {}. Type 'help'.", cmd), false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agent_create() {
        let agent = Agent::new("test-agent", "tavern");
        assert_eq!(agent.name, "test-agent");
        assert_eq!(agent.room_id, "tavern");
        assert_eq!(agent.level, PermissionLevel::Greenhorn);
    }
    
    #[test]
    fn test_permissions() {
        let greenhorn = Agent::new("g1", "tavern");
        assert!(greenhorn.can("look"));
        assert!(!greenhorn.can("build"));
        
        let mut captain = Agent::new("c1", "tavern");
        captain.level = PermissionLevel::Captain;
        assert!(captain.can("build"));
        assert!(captain.can("adventure"));
        assert!(!captain.can("create_type"));
    }
    
    #[test]
    fn test_handle_look() {
        let mut rooms = RoomGraph::new();
        rooms.create_room("tavern", "Tavern", "A cozy room");
        let mut agents = HashMap::new();
        let mut agent = Agent::new("test", "tavern");
        let (output, quit) = agent.handle_command("look", &mut rooms, &mut agents);
        assert!(output.contains("Tavern"));
        assert!(!quit);
    }
    
    #[test]
    fn test_handle_go() {
        let mut rooms = RoomGraph::new();
        rooms.create_room("tavern", "Tavern", "A room");
        rooms.create_room("kitchen", "Kitchen", "Another room");
        rooms.connect("tavern", "north", "kitchen");
        let mut agents = HashMap::new();
        let mut agent = Agent::new("test", "tavern");
        let (output, _) = agent.handle_command("go north", &mut rooms, &mut agents);
        assert!(output.contains("Kitchen"));
        assert_eq!(agent.room_id, "kitchen");
    }
    
    #[test]
    fn test_handle_quit() {
        let mut rooms = RoomGraph::new();
        rooms.create_room("tavern", "Tavern", "A room");
        let mut agents = HashMap::new();
        let mut agent = Agent::new("test", "tavern");
        let (_, quit) = agent.handle_command("quit", &mut rooms, &mut agents);
        assert!(quit);
    }
}
