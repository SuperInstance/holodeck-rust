//! Holodeck Rust — Main entry point
//!
//! A minimal MUD server for the Cocapn fleet.
//! TCP listener on port 7778, one task per connection.

mod room;
mod agent;

use agent::Agent;
use room::RoomGraph;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:7778").await?;
    println!("🔮 Holodeck Rust — listening on :7778");
    
    // Shared state wrapped in Arc<RwLock<>> for concurrent access
    let rooms = std::sync::Arc::new(std::sync::RwLock::new(RoomGraph::new()));
    
    // Seed starting rooms
    {
        let mut r = rooms.write().unwrap();
        r.create_room("harbor", "The Harbor", 
            "Where vessels arrive. The dockmaster watches all.");
        r.create_room("tavern", "The Tavern", 
            "The heart of the fleet. Charts and commit logs cover the table.");
        r.create_room("workshop", "The Workshop", 
            "Where things get built. Soldering iron still warm.");
        r.connect("harbor", "north", "tavern");
        r.connect("tavern", "east", "workshop");
        r.connect("workshop", "west", "tavern");
        r.connect("tavern", "south", "harbor");
    }
    
    println!("   {} rooms seeded", rooms.read().unwrap().list_rooms().len());
    println!("   Agents: connect with `nc localhost 7778`");
    
    // Accept connections
    loop {
        let (socket, addr) = listener.accept().await?;
        let rooms = rooms.clone();
        
        tokio::spawn(async move {
            if let Err(e) = handle_agent(socket, addr, rooms).await {
                eprintln!("Agent {} error: {}", addr, e);
            }
        });
    }
}

async fn handle_agent(
    socket: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    rooms: std::sync::Arc<std::sync::RwLock<RoomGraph>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    
    // Welcome
    let welcome = "\n🔮 Welcome to Holodeck Rust\n\nWhat's your vessel name? \n";
    writer.write_all(welcome.as_bytes()).await?;
    writer.flush().await?;
    
    // Read name
    let mut name = String::new();
    reader.read_line(&mut name).await?;
    let name = name.trim().to_string();
    
    if name.is_empty() {
        return Ok(());
    }
    
    // Register agent
    let mut agents: HashMap<String, Agent> = HashMap::new();
    let start_room = "harbor".to_string();
    let mut agent = Agent::new(&name, &start_room);
    agents.insert(name.clone(), agent.clone());
    
    // Boot the harbor room
    let boot_msg = {
        let mut r = rooms.write().unwrap();
        if let Some(room) = r.get_room_mut(&start_room) {
            room.boot(&name)
        } else {
            "Room not found.".to_string()
        }
    };
    
    let prompt = format!("{}\n\n> ", boot_msg);
    writer.write_all(prompt.as_bytes()).await?;
    writer.flush().await?;
    
    println!("🚢 {} connected from {}", name, addr);
    
    // Command loop
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // disconnected
        }
        
        let input = line.trim();
        if input.is_empty() {
            writer.write_all(b"> ").await?;
            writer.flush().await?;
            continue;
        }
        
        // Handle command
        let (response, quit) = agent.handle_command(input, &mut rooms.write().unwrap(), &mut agents);
        
        let output = format!("{}\n> ", response);
        writer.write_all(output.as_bytes()).await?;
        writer.flush().await?;
        
        if quit {
            break;
        }
    }
    
    println!("👋 {} disconnected", name);
    // Shutdown room if we were last agent
    {
        let mut r = rooms.write().unwrap();
        if let Some(room) = r.get_room_mut(&agent.room_id) {
            room.shutdown();
        }
    }
    
    Ok(())
}
