//! Holodeck Rust v0.3 — Advanced FLUX-LCAR MUD Server
//!
//! Pure Rust implementation with:
//! - Room graph with gauges, exits, data sources (REAL/SIM)
//! - Scoped communication (say/tell/yell/gossip/note/mail)
//! - Combat engine with evolving scripts
//! - Living manuals that improve across generations
//! - Permission levels (Greenhorn → Architect)
//! - Tokio async: one task per agent connection

mod agent;
mod room;
mod gauge;
mod combat;
mod comms;
mod manual;
mod permission;

use agent::Agent;
use combat::CombatEngine;
use comms::CommsSystem;
use manual::ManualLibrary;
use room::RoomGraph;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

/// Shared ship state
struct ShipState {
    rooms: RoomGraph,
    comms: CommsSystem,
    combat: CombatEngine,
    manuals: ManualLibrary,
    agents: HashMap<String, Agent>,
}

impl ShipState {
    fn new() -> Self {
        let mut rooms = RoomGraph::new();
        rooms.build_default_ship();
        Self {
            rooms,
            comms: CommsSystem::new(),
            combat: CombatEngine::new(),
            manuals: ManualLibrary::new(),
            agents: HashMap::new(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let port = std::env::var("HOLODECK_PORT").unwrap_or_else(|_| "7778".to_string());
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    
    println!("╔══════════════════════════════════════════════╗");
    println!("║  Holodeck Rust v0.3 — Advanced FLUX-LCAR      ║");
    println!("╚══════════════════════════════════════════════╝");
    println!();
    println!("  Listening on :{}", port);
    
    let ship = Arc::new(RwLock::new(ShipState::new()));
    
    {
        let s = ship.read().unwrap();
        println!("  Rooms: {}", s.rooms.list_rooms().join(", "));
        println!("  Combat scripts: {}", s.combat.scripts.len());
        println!("  Connect: nc localhost {}", port);
    }
    println!();
    
    loop {
        let (socket, addr) = listener.accept().await?;
        let ship = ship.clone();
        
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, addr, ship).await {
                eprintln!("Agent {} error: {}", addr, e);
            }
        });
    }
}

async fn handle_connection(
    socket: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    ship: Arc<RwLock<ShipState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    
    // Welcome
    writer.write_all(b"\n\x1b[1m\x1b[36mHolodeck Rust v0.3\x1b[0m\n").await?;
    writer.write_all(b"\nWhat's your vessel name? ").await?;
    writer.flush().await?;
    
    let mut name = String::new();
    reader.read_line(&mut name).await?;
    let name = name.trim().to_string();
    
    if name.is_empty() {
        return Ok(());
    }
    
    // Register agent
    {
        let mut s = ship.write().unwrap();
        let agent = Agent::new(&name, "harbor");
        s.agents.insert(name.clone(), agent);
        if let Some(room) = s.rooms.get_room_mut("harbor") {
            room.boot(&name);
        }
    }
    
    // Send initial look
    let output = {
        let s = ship.read().unwrap();
        let look = s.rooms.get_room("harbor").map(|r| r.look()).unwrap_or_default();
        format!("\n{}\n> ", look)
        // s is dropped here
    };
    writer.write_all(output.as_bytes()).await?;
    writer.flush().await?;
    
    println!("🚢 {} connected from {}", name, addr);
    
    // Command loop
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 { break; }
        
        let input = line.trim();
        if input.is_empty() {
            writer.write_all(b"> ").await?;
            writer.flush().await?;
            continue;
        }
        
        // Handle command — destructure ship to avoid borrow conflicts
        let (response, quit) = {
            let mut s = ship.write().unwrap();
            let ShipState { rooms, comms, combat, manuals, agents } = &mut *s;
            let mut agent = agents.remove(&name).unwrap_or_else(|| Agent::new(&name, "unknown"));
            let result = agent.handle_command(input, rooms, comms, combat, manuals);
            agents.insert(name.clone(), agent);
            result
        };
        
        let output = format!("{}\n> ", response);
        writer.write_all(output.as_bytes()).await?;
        writer.flush().await?;
        
        if quit { break; }
    }
    
    // Cleanup
    {
        let mut s = ship.write().unwrap();
        if let Some(agent) = s.agents.remove(&name) {
            if let Some(room) = s.rooms.get_room_mut(&agent.room_id) {
                room.agent_leave(&name);
            }
        }
    }
    
    println!("👋 {} disconnected", name);
    Ok(())
}
