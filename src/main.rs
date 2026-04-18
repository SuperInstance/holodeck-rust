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
mod evolution;
mod director;
mod plato_bridge;

use agent::Agent;
use combat::CombatEngine;
use comms::CommsSystem;
use manual::ManualLibrary;
use npc::{default_npcs, NpcConfig};
use npc_refresh::NpcRefresh;
use games::PokerGame;
use room::RoomGraph;
use plato_bridge::PlatoBridge;
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
    evolver: evolution::ScriptEvolver,
    ai_director: Option<director::DirectorState>,
    agent_actions: Vec<String>,  // track what the agent did for director context
    plato: PlatoBridge,          // tile + sentiment bridge to plato-torch
}

impl ShipState {
    fn new() -> Self {
        let mut rooms = RoomGraph::new();
        rooms.build_default_ship();
        let deepinfra_key = std::env::var("DEEPINFRA_API_KEY").ok();
        let mut combat = CombatEngine::new();
        let evolver = evolution::ScriptEvolver::new();
        evolution::ScriptEvolver::seed_defaults(&mut combat);
        Self {
            rooms,
            comms: CommsSystem::new(),
            combat,
            manuals: ManualLibrary::new(),
            agents: HashMap::new(),
            npcs: default_npcs(),
            deepinfra_key,
            npc_refresh: NpcRefresh::new(),
            poker: PokerGame::new(),
            ten_forward_chat: Vec::new(),
            active_program: None,
            evolver,
            ai_director: None,
            agent_actions: Vec::new(),
            plato: PlatoBridge::new("/tmp/plato-tiles"),
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
    
    // Background combat ticker — ticks every 30 seconds
    {
        let ship = ship.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let mut s = ship.write().unwrap();
                // Tick combat for each room with gauges
                let room_ids: Vec<String> = s.rooms.rooms.keys().cloned().collect();
                for room_id in &room_ids {
                    if let Some(room) = s.rooms.get_room(room_id) {
                        if !room.gauges.is_empty() {
                            let gauges = room.gauges.clone();
                            s.combat.tick(room_id, &gauges);
                        }
                    }
                }
                // Auto-evolve every 10 ticks
                if s.combat.tick_count.is_multiple_of(10) && s.combat.tick_count > 0 {
                    let room_gauges: HashMap<String, HashMap<String, _>> = s.rooms.rooms.iter()
                        .map(|(id, room)| (id.clone(), room.gauges.clone()))
                        .collect();
                    let ShipState { evolver, combat, .. } = &mut *s;
                    let mutations = evolver.evolve(combat, &room_gauges);
                    if !mutations.is_empty() {
                        println!("🧬 Evolution: {}", mutations.join(", "));
                    }
                }
            }
        });
    }
    
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
    
    // Welcome + MOTD
    writer.write_all(b"\n\x1b[1m\x1b[36mHolodeck Rust v0.3\x1b[0m\n").await?;
    let _ = writer.write_all(b"\x1b[33mWelcome to the Holodeck. Rooms are live systems.\n").await;
    writer.write_all(b"Commands: help | look | go <exit> | say <msg> | fleet | scripts\x1b[0m\n").await?;
    writer.write_all(b"\nWhat's your vessel name? ").await?;
    writer.flush().await?;
    
    let mut name = String::new();
    reader.read_line(&mut name).await?;
    let name = name.trim().to_string();
    
    // Filter HTTP requests and invalid names
    if name.is_empty() || name.starts_with("GET ") || name.starts_with("POST ") || name.starts_with("OPTIONS ") {
        return Ok(());
    }
    
    // Max name length to prevent abuse
    let name = if name.len() > 32 { name[..32].to_string() } else { name };
    
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
        if ["programs", "run", "tickprog", "adjust", "progstatus", "endprog", "director", "sentiment", "tiles", "flushtiles"].contains(&cmd_lower.as_str()) {
            let response = {
                let mut s = ship.write().unwrap();
                let ShipState { agents, active_program, ai_director, agent_actions, plato, .. } = &mut *s;
                let agent = agents.get(&name).unwrap();
                let in_holodeck = agent.room_id == "holodeck";
                if !in_holodeck && cmd_lower != "programs" && cmd_lower != "sentiment" && cmd_lower != "tiles" && cmd_lower != "flushtiles" {
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
                                    // If director is active, inject its event
                                    if let Some(ref dir) = ai_director {
                                        let _system_prompt = dir.system_prompt();
                                        let _state_prompt = dir.state_prompt(prog, agent_actions);
                                        // We'll call the API async below
                                        // For now, just tick normally (director events added via 'directorevent' command)
                                    }
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
                                    Some(ref mut prog) => {
                                        let result = prog.adjust(&gauge, delta);
                                        agent_actions.push(format!("adjust {} {:+.1}", gauge, delta));
                                        if agent_actions.len() > 20 { agent_actions.drain(0..5); }
                                        result
                                    },
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
                            *ai_director = None;
                            "Holodeck program ended. Director dismissed.".to_string()
                        },
                        "director" => {
                            let style_name = parts.get(1).unwrap_or(&"").to_string();
                            if style_name.is_empty() {
                                match ai_director {
                                    Some(d) => format!("Director: {:?} (brutality={:.1}, creativity={:.1}, empathy={:.1})",
                                        d.style, d.brutality, d.creativity, d.empathy),
                                    None => "No director active. Usage: director <adversary|teacher|storyteller|trickster>".to_string(),
                                }
                            } else {
                                let style = match style_name.as_str() {
                                    "adversary" => Some(director::DirectorStyle::Adversary),
                                    "teacher" => Some(director::DirectorStyle::Teacher),
                                    "storyteller" => Some(director::DirectorStyle::Storyteller),
                                    "trickster" => Some(director::DirectorStyle::Trickster),
                                    _ => None,
                                };
                                match style {
                                    Some(s) => {
                                        *ai_director = Some(director::DirectorState::new(s));
                                        format!("Director activated: {:?}. The simulation now has a mind of its own.", s)
                                    },
                                    None => "Unknown style. Options: adversary, teacher, storyteller, trickster".to_string()
                                }
                            }
                        },
                        "flushtiles" => {
                            let count = plato.stats().1;
                            plato.flush();
                            format!("Flushed {} tiles to disk.", count)
                        },
                        "sentiment" => {
                            let room = agent.room_id.clone();
                            match plato.get_sentiment(&room) {
                                Some(sent) => {
                                    let bias = sent.bias();
                                    format!(
                                        "Room Sentiment [{}]\n  energy:     {:.2}\n  flow:       {:.2}\n  frustration:{:.2}\n  discovery:  {:.2}\n  tension:    {:.2}\n  confidence: {:.2}\n\nBias: explore={:.2} safe={:.2} novel={:.2}\nJEPA: {:?}",
                                        room, sent.energy, sent.flow, sent.frustration,
                                        sent.discovery, sent.tension, sent.confidence,
                                        bias.explore_bias, bias.safe_bias, bias.novel_bias,
                                        sent.to_jepa_vector()
                                    )
                                },
                                None => format!("No sentiment data for room '{}'. Do something first!", room),
                            }
                        },
                        "tiles" => {
                            let room = agent.room_id.clone();
                            let tiles = plato.room_tiles(&room, 5);
                            if tiles.is_empty() {
                                format!("No tiles recorded for room '{}'.", room)
                            } else {
                                let header = format!("Recent tiles in {} ({} shown):", room, tiles.len());
                                let body: Vec<String> = tiles.iter().map(|t| {
                                    format!("  {} → {} ({:.1}) [{}]", t.agent, t.action, t.reward, t.outcome)
                                }).collect();
                                format!("{}\n{}\nStats: {} events, {} tiles buffered, {} rooms tracked",
                                    header, body.join("\n"), plato.stats().0, plato.stats().1, plato.stats().2)
                            }
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
                let ShipState { poker, ten_forward_chat, agents, active_program: _, ai_director: _, agent_actions: _, .. } = &mut *s;
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
                                name.to_string()
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

        // Handle evolve and scripts globally
        if cmd_lower == "evolve" || cmd_lower == "scripts" {
            let response = if cmd_lower == "evolve" {
                let room_gauges: HashMap<String, HashMap<String, crate::gauge::Gauge>> = {
                    let s = ship.read().unwrap();
                    s.rooms.rooms.iter().map(|(id, room)| {
                        (id.clone(), room.gauges.clone())
                    }).collect()
                };
                let (mutations, gen) = {
                    let mut s = ship.write().unwrap();
                    let ShipState { evolver, combat, .. } = &mut *s;
                    let gen = evolver.generation;
                    let m = evolver.evolve(combat, &room_gauges);
                    (m, gen)
                };
                if mutations.is_empty() { "No mutations this cycle.".to_string() }
                else { format!("Evolution gen {}:\n{}", gen, mutations.join("\n")) }
            } else {
                let s = ship.read().unwrap();
                let script_list: Vec<String> = s.combat.scripts.iter()
                    .map(|sc| format!("  {} (gen {}, pri {}) — {} conditions",
                        sc.name, sc.generation, sc.priority, sc.conditions.len()))
                    .collect();
                let stats = s.evolver.stats(&s.combat);
                format!("{}\n{}", stats, script_list.join("\n"))
            };
            let output = format!("{}\n> ", response);
            writer.write_all(output.as_bytes()).await?;
            writer.flush().await?;
            continue;
        }

        // Handle fleet command — queries keeper API
        if cmd_lower == "fleet" {
            let response = match reqwest::get("http://localhost:8900/health").await {
                Ok(resp) => {
                    match resp.json::<serde_json::Value>().await {
                        Ok(v) => {
                            let version = v.get("version").and_then(|v| v.as_str()).unwrap_or("?");
                            let agents = v.get("agents").and_then(|v| v.as_u64()).unwrap_or(0);
                            let calls = v.get("api_calls").and_then(|v| v.as_u64()).unwrap_or(0);
                            format!("🏠 Lighthouse Keeper v{}\n  Vessels: {} | API calls: {}", version, agents, calls)
                        }
                        Err(e) => format!("Parse error: {}", e)
                    }
                }
                Err(e) => format!("Keeper unreachable: {}", e)
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
                let (new_npcs, cost, failures) = npc_refresh::refresh_npcs_async(npc_configs, &snapshots, &api_key).await;
                let mut s = ship.write().unwrap();
                s.npcs = new_npcs;
                s.npc_refresh.refresh_count += 1;
                s.npc_refresh.total_cost += cost;
                s.npc_refresh.failures += failures;
                format!("Refreshed ${:.4} ({} failures). Talk with 'npc'.", cost, failures)
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
            let ShipState { rooms, comms, combat, manuals, agents, npcs, deepinfra_key: _, npc_refresh: _, poker: _, ten_forward_chat: _, active_program: _, evolver: _, ai_director: _, agent_actions: _, plato } = &mut *s;
            let mut agent = agents.remove(&name).unwrap_or_else(|| Agent::new(&name, "unknown"));
            let old_room = agent.room_id.clone();
            let (result, quit) = agent.handle_command(input, rooms, comms, combat, manuals, npcs);
            let new_room = agent.room_id.clone();
            
            // Record tile for this action
            let reward = if result.contains("success") || result.contains("done") || result.contains("moved") { 1.0 } else { 0.0 };
            let is_discovery = result.contains("discovered") || result.contains("found") || result.contains("new");
            plato.record_event(&new_room, &name, &cmd_lower, &result, reward, is_discovery);
            
            // If agent moved rooms, record that too
            if old_room != new_room {
                plato.record_event(&new_room, &name, "arrive", &format!("from {}", old_room), 0.5, false);
            }
            
            agents.insert(name.clone(), agent);
            (result, quit)
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
