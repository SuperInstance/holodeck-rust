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
mod npc;
mod npc_refresh;
mod games;
mod holodeck;

use agent::Agent;
use combat::CombatEngine;
use comms::CommsSystem;
use manual::ManualLibrary;
use npc::{default_npcs, NpcConfig};
use npc_refresh::NpcRefresh;
use games::PokerGame;
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
    npcs: Vec<NpcConfig>,
    deepinfra_key: Option<String>,
    npc_refresh: NpcRefresh,
    poker: PokerGame,
    ten_forward_chat: Vec<(String, String)>,  // (agent, message)
    active_program: Option<holodeck::ActiveProgram>,
}

impl ShipState {
    fn new() -> Self {
        let mut rooms = RoomGraph::new();
        rooms.build_default_ship();
        let deepinfra_key = std::env::var("DEEPINFRA_API_KEY").ok();
        Self {
            rooms,
            comms: CommsSystem::new(),
            combat: CombatEngine::new(),
            manuals: ManualLibrary::new(),
            agents: HashMap::new(),
            npcs: default_npcs(),
            deepinfra_key,
            npc_refresh: NpcRefresh::new(),
            poker: PokerGame::new(),
            ten_forward_chat: Vec::new(),
            active_program: None,
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
        
        let cmd_lower = input.split_whitespace().next().unwrap_or("").to_lowercase();

        // Handle holodeck program commands (only in holodeck room)
        if ["programs", "run", "tickprog", "adjust", "progstatus", "endprog"].contains(&cmd_lower.as_str()) {
            let response = {
                let mut s = ship.write().unwrap();
                let ShipState { agents, active_program, .. } = &mut *s;
                let agent = agents.get(&name).unwrap();
                let in_holodeck = agent.room_id == "holodeck";
                if !in_holodeck && cmd_lower != "programs" {
                    "Program commands only work in the Holodeck. Go there first: go holodeck".to_string()
                } else {
                    let parts: Vec<&str> = input.splitn(3, ' ').collect();
                    match cmd_lower.as_str() {
                        "programs" => {
                            let list = holodeck::HolodeckProgram::list_programs();
                            format!("Available programs:\n{}", list.join("\n"))
                        },
                        "run" => {
                            let prog_name = parts.get(1).unwrap_or(&"").to_string();
                            if prog_name.is_empty() {
                                "Usage: run <program-name>".to_string()
                            } else {
                                match holodeck::HolodeckProgram::catalog().into_iter().find(|p| p.name == prog_name) {
                                    Some(prog) => {
                                        let status = prog.objective.clone();
                                        *active_program = Some(holodeck::ActiveProgram::new(prog));
                                        format!("Holodeck program loaded: {}\nObjective: {}\nType 'tickprog' to advance, 'adjust <gauge> <delta>' to intervene, 'progstatus' to check.", prog_name, status)
                                    },
                                    None => format!("Unknown program '{}'. Type 'programs' to see available.", prog_name)
                                }
                            }
                        },
                        "tickprog" => {
                            match active_program {
                                Some(ref mut prog) => {
                                    let msgs = prog.tick();
                                    msgs.join("\n")
                                },
                                None => "No program running. Type 'run <name>' to start one.".to_string()
                            }
                        },
                        "adjust" => {
                            let gauge = parts.get(1).unwrap_or(&"").to_string();
                            let delta: f64 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                            if gauge.is_empty() || delta == 0.0 {
                                "Usage: adjust <gauge> <delta>  (e.g., adjust reactor_temp -10)".to_string()
                            } else {
                                match active_program {
                                    Some(ref mut prog) => prog.adjust(&gauge, delta),
                                    None => "No program running.".to_string()
                                }
                            }
                        },
                        "progstatus" => {
                            match active_program {
                                Some(ref prog) => prog.status(),
                                None => "No program running.".to_string()
                            }
                        },
                        "endprog" => {
                            *active_program = None;
                            "Holodeck program ended.".to_string()
                        },
                        _ => "Unknown.".to_string()
                    }
                }
            };
            let output = format!("{}\n> ", response);
            writer.write_all(output.as_bytes()).await?;
            writer.flush().await?;
            continue;
        }

        // Handle Ten Forward social commands
        if ["join", "deal", "hand", "flop", "turn", "river", "bet", "fold", "table", "chat", "chatlog"].contains(&cmd_lower.as_str()) {
            let response = {
                let mut s = ship.write().unwrap();
                let ShipState { poker, ten_forward_chat, agents, .. } = &mut *s;
                let agent = agents.get(&name).unwrap();
                let in_tf = agent.room_id == "ten-forward";
                if !in_tf {
                    "Social commands only work in Ten Forward. Go there first: go ten-forward".to_string()
                } else {
                    let parts: Vec<&str> = input.splitn(3, ' ').collect();
                    match cmd_lower.as_str() {
                        "join" => poker.join(&name),
                        "deal" => poker.deal(),
                        "hand" => poker.show_hand(&name),
                        "flop" => poker.flop(),
                        "turn" => poker.turn(),
                        "river" => poker.river(),
                        "bet" => {
                            let amount: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                            if amount == 0 { "Usage: bet <amount>".to_string() }
                            else { poker.bet(&name, amount) }
                        },
                        "fold" => poker.fold(&name),
                        "table" => poker.show_table(),
                        "chat" => {
                            let msg = parts.get(1).unwrap_or(&"").to_string();
                            if msg.is_empty() { "Say something: chat <message>".to_string() }
                            else {
                                ten_forward_chat.push((name.clone(), msg.clone()));
                                if ten_forward_chat.len() > 50 { ten_forward_chat.drain(0..10); }
                                format!("{}", name)
                            }
                        },
                        "chatlog" => {
                            let recent: Vec<String> = ten_forward_chat.iter().rev().take(10)
                                .map(|(a, m)| format!("{}: {}", a, m)).collect();
                            if recent.is_empty() { "Quiet night. Be the first to chat.".to_string() }
                            else { recent.into_iter().rev().collect::<Vec<_>>().join("\n") }
                        },
                        _ => "Unknown command.".to_string(),
                    }
                }
            };
            let output = format!("{}\n> ", response);
            writer.write_all(output.as_bytes()).await?;
            writer.flush().await?;
            continue;
        }

        // Handle refreshnpcs specially (needs api key + refresh state)
        if input == "refreshnpcs" {
            let (snapshots, npc_configs, key) = {
                let s = ship.read().unwrap();
                let snapshots: Vec<(String, Vec<String>)> = s.rooms.rooms.iter().map(|(id, room)| {
                    let readings: Vec<String> = room.gauges.values().map(|g| {
                        format!("{}: {:.1}{}", g.name, g.value, g.unit)
                    }).collect();
                    (id.clone(), readings)
                }).collect();
                (snapshots, s.npcs.clone(), s.deepinfra_key.clone())
            }; // read lock released

            let response = if let Some(api_key) = key {
                match npc_refresh::refresh_npcs_async(npc_configs, &snapshots, &api_key).await {
                    (new_npcs, cost, failures) => {
                        let mut s = ship.write().unwrap();
                        s.npcs = new_npcs;
                        s.npc_refresh.refresh_count += 1;
                        s.npc_refresh.total_cost += cost;
                        s.npc_refresh.failures += failures;
                        format!("Refreshed ${:.4} ({} failures). Talk with 'npc'.", cost, failures)
                    }
                }
            } else {
                "No DEEPINFRA_API_KEY set.".to_string()
            };
            let output = format!("{}\n> ", response);
            writer.write_all(output.as_bytes()).await?;
            writer.flush().await?;
            continue;
        }

        // Handle command — destructure ship to avoid borrow conflicts
        let (response, quit) = {
            let mut s = ship.write().unwrap();
            let ShipState { rooms, comms, combat, manuals, agents, npcs, deepinfra_key: _, npc_refresh: _, poker: _, ten_forward_chat: _, active_program: _ } = &mut *s;
            let mut agent = agents.remove(&name).unwrap_or_else(|| Agent::new(&name, "unknown"));
            let result = agent.handle_command(input, rooms, comms, combat, manuals, npcs);
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
