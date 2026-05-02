//! Agent — the entity that moves through rooms and issues commands.

use crate::comms::CommsSystem;
use crate::combat::CombatEngine;
use crate::manual::ManualLibrary;
use crate::permission::Permission;
use crate::room::RoomGraph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    pub room_id: String,
    pub permission: Permission,
    pub energy: u32,
    pub connected_at: i64,
}

impl Agent {
    pub fn new(name: &str, room_id: &str) -> Self {
        Self {
            name: name.to_string(),
            room_id: room_id.to_string(),
            permission: Permission::Crew,
            energy: 1000,
            connected_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Process and execute a user command, updating agent state as needed.
    ///
    /// # Purpose
    /// This is the central command dispatcher for agent interactions. It parses
    /// the input string, identifies the command and arguments, and delegates to
    /// the appropriate command handler method. Commands can affect agent state,
    /// room state, communication systems, combat engine, manuals, and NPCs.
    ///
    /// # Arguments
    /// * `input` - The raw command string from the user (e.g., "go north", "say hello", "look")
    /// * `rooms` - Mutable reference to the room graph, which may be modified by movement commands
    /// * `comms` - Mutable reference to the communications system, used by say/tell/yell/gossip/note/mail
    /// * `combat` - Mutable reference to the combat engine, used by tick/alert/script commands
    /// * `manuals` - Mutable reference to the manual library, used by manual/feedback commands
    /// * `npcs` - Slice of NPC configurations, used to list NPCs in the current room
    ///
    /// # Returns
    /// A tuple of `(String, bool)` where:
    /// - The first element is the response message to display to the user
    /// - The second element is a quit flag: `true` if the agent should disconnect, `false` otherwise
    ///
    /// # Errors
    /// This function does not return errors directly. Unknown commands return an error message
    /// in the response tuple. Commands with insufficient permissions return an appropriate message.
    ///
    /// # Supported Commands
    /// - `look` or `l`: Look around the current room
    /// - `go <direction>`: Move to an adjacent room (also accepts `move`, `walk`)
    /// - `say <message>`: Speak to everyone in the current room (also accepts `"`)
    /// - `tell <agent> <message>`: Send a private message to another agent
    /// - `yell <message>`: Broadcast to all agents on the ship
    /// - `gossip <message>`: Broadcast to all agents in the fleet
    /// - `who`: List agents in the current room
    /// - `status`: Show ship status, alerts, and agent info
    /// - `tick`: Run combat tick (requires appropriate permissions)
    /// - `alert [level]`: Set or view alert level (green/yellow/red)
    /// - `note <content>`: Write a note on the current room's wall
    /// - `notes`: Read notes on the current room's wall
    /// - `mail`: Check mailbox for messages
    /// - `gauge <name> <value>`: Update a gauge in the current room
    /// - `sim`/`real`: Switch between simulation and real sensor data modes
    /// - `manual`: Read the living manual for this room
    /// - `feedback <1-5> <comment>`: Rate the manual and provide feedback
    /// - `script <description>`: Add a combat script
    /// - `npc` or `talk`: List NPCs in the current room
    /// - `permission` or `perms`: Show your permission level
    /// - `map`: Display ship map with current location highlighted
    /// - `help` or `?`: Show help with all available commands
    /// - `quit` or `exit` or `q`: Disconnect from the holodeck
    ///
    /// # Usage Example
    /// ```no_run
    /// # use holodeck_rust::agent::Agent;
    /// # use holodeck_rust::room::RoomGraph;
    /// # use holodeck_rust::comms::CommsSystem;
    /// # use holodeck_rust::combat::CombatEngine;
    /// # use holodeck_rust::manual::ManualLibrary;
    /// # use holodeck_rust::npc::NpcConfig;
    /// let mut agent = Agent::new("TestAgent", "harbor");
    /// let mut rooms = RoomGraph::new();
    /// let mut comms = CommsSystem::new();
    /// let mut combat = CombatEngine::new();
    /// let mut manuals = ManualLibrary::new();
    /// let npcs: Vec<NpcConfig> = vec![];
    ///
    /// // Execute a look command
    /// let (response, quit) = agent.handle_command("look", &mut rooms, &mut comms, &mut combat, &mut manuals, &npcs);
    /// assert_eq!(quit, false);
    ///
    /// // Execute a movement command
    /// let (response, quit) = agent.handle_command("go bridge", &mut rooms, &mut comms, &mut combat, &mut manuals, &npcs);
    /// assert_eq!(agent.room_id, "bridge");
    /// ```
    pub fn handle_command(
        &mut self,
        input: &str,
        rooms: &mut RoomGraph,
        comms: &mut CommsSystem,
        combat: &mut CombatEngine,
        manuals: &mut ManualLibrary,
        npcs: &[crate::npc::NpcConfig],
    ) -> (String, bool) {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let args = parts.get(1).unwrap_or(&"");

        match cmd.as_str() {
            "look" | "l" => (self.cmd_look(rooms), false),
            "go" | "move" | "walk" => self.cmd_go(args, rooms),
            "say" | "\"" => (self.cmd_say(args, comms), false),
            "tell" => (self.cmd_tell(args, comms), false),
            "yell" | "broadcast" => (self.cmd_yell(args, comms), false),
            "gossip" => (self.cmd_gossip(args, comms), false),
            "who" => (self.cmd_who(rooms), false),
            "status" => (self.cmd_status(rooms, combat), false),
            "tick" => self.cmd_tick(rooms, combat),
            "alert" => self.cmd_alert(args, combat),
            "note" => (self.cmd_note(args, comms), false),
            "notes" => (self.cmd_read_notes(comms), false),
            "mail" | "mailbox" => (self.cmd_check_mail(comms), false),
            "gauge" => self.cmd_update_gauge(args, rooms),
            "sim" => (self.cmd_sim_mode(rooms), false),
            "real" => (self.cmd_real_mode(rooms), false),
            "manual"            => (self.cmd_manual(manuals), false),
            "npc" | "talk" => (self.cmd_list_npcs(npcs, rooms), false),
            "refresh" => ("Use 'refreshnpcs' to refresh NPC dialogue from Seed-2.0-Mini.".to_string(), false),
            "feedback" => (self.cmd_feedback(args, manuals), false),
            "script" => (self.cmd_add_script(args, combat), false),
            "permission" | "perms" => (format!("Your permission level: {}", self.permission.name()), false),
            "map" => (self.cmd_map(rooms), false),
            "help" | "?" => (self.cmd_help(), false),
            "quit" | "exit" | "q" => (self.cmd_quit(rooms), true),
            "" => ("> ".to_string(), false),
            _ => (format!("Unknown command: {}. Type 'help' for commands.", cmd), false),
        }
    }

    fn cmd_look(&self, rooms: &RoomGraph) -> String {
        self.get_current_room(rooms)
            .map(|room| room.look())
            .unwrap_or_else(|| self.format_nowhere_message())
    }

    fn get_current_room<'a>(&'a self, rooms: &'a RoomGraph) -> Option<&'a crate::room::Room> {
        rooms.get_room(&self.room_id)
    }

    fn format_nowhere_message(&self) -> String {
        "You are nowhere.".to_string()
    }

    fn cmd_go(&mut self, args: &str, rooms: &mut RoomGraph) -> (String, bool) {
        let direction = args.trim();
        if direction.is_empty() {
            return ("Go where? Specify a direction.".to_string(), false);
        }
        let target = rooms.get_room(&self.room_id)
            .and_then(|r| r.exits.get(direction).cloned());
        match target {
            Some(target_id) => {
                if let Some(room) = rooms.get_room_mut(&self.room_id) {
                    room.agent_leave(&self.name);
                }
                self.room_id = target_id.clone();
                if let Some(room) = rooms.get_room_mut(&target_id) {
                    let boot_msg = room.boot(&self.name);
                    let look = room.look();
                    (format!("{}\n\n{}", boot_msg, look), false)
                } else {
                    ("Room exists but couldn't be entered.".to_string(), false)
                }
            }
            None => (format!("No exit '{}' from here.", direction), false),
        }
    }

    // Actually, cmd_go returns different types per branch. Let me use a single return.
    fn cmd_say(&self, args: &str, comms: &mut CommsSystem) -> String {
        let msg = comms.say(&self.name, &self.room_id, args);
        format!("You say: {}", msg.content)
    }

    fn cmd_tell(&self, args: &str, comms: &mut CommsSystem) -> String {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.len() < 2 {
            return "Usage: tell <agent> <message>".to_string();
        }
        let target = parts[0];
        let message = parts[1];
        comms.tell(&self.name, target, message);
        format!("You tell {}: {}", target, message)
    }

    fn cmd_yell(&self, args: &str, comms: &mut CommsSystem) -> String {
        if !self.permission.can_yell() {
            return "Insufficient permission to yell.".to_string();
        }
        comms.yell(&self.name, args);
        format!("[BRIDGE] {} yells: {}", self.name, args)
    }

    fn cmd_gossip(&self, args: &str, comms: &mut CommsSystem) -> String {
        if !self.permission.can_gossip() {
            return "Insufficient permission to gossip.".to_string();
        }
        comms.gossip(&self.name, args);
        format!("[FLEET] {} gossips: {}", self.name, args)
    }

    fn cmd_who(&self, rooms: &RoomGraph) -> String {
        let mut out = String::from("Agents in this room:\n");
        if let Some(room) = rooms.get_room(&self.room_id) {
            for agent in &room.agents {
                out.push_str(&format!("  {}\n", agent));
            }
        }
        if out == "Agents in this room:\n" {
            out.push_str("  (just you)\n");
        }
        out
    }

    fn cmd_status(&self, rooms: &RoomGraph, combat: &CombatEngine) -> String {
        let room_count = rooms.rooms.len();
        let booted = rooms.rooms.values().filter(|r| r.booted).count();
        let alert = combat.fleet_alert_level();
        format!(
            "Ship Status:\n  Rooms: {} ({} booted)\n  Fleet Alert: {}\n  Tick: {}\n  Scripts: {}\n  You: {} @ {} ({})",
            room_count, booted, alert, combat.tick_count, combat.scripts.len(),
            self.name, self.room_id, self.permission.name()
        )
    }

    fn cmd_tick(&self, rooms: &mut RoomGraph, combat: &mut CombatEngine) -> (String, bool) {
        if !self.permission.can_run_combat() {
            return ("Insufficient permission for combat tick.".to_string(), false);
        }
        let mut results = Vec::new();
        for (room_id, room) in &rooms.rooms {
            if room.booted {
                let tick = combat.tick(room_id, &room.gauges);
                results.push(format!("  {}: {} alerts, {} scripts fired", room_id, tick.alerts.len(), tick.scripts_fired.len()));
            }
        }
        (format!("Combat Tick {}:\n{}\nFleet Alert: {}", combat.tick_count, results.join("\n"), combat.fleet_alert_level()), false)
    }

    fn cmd_alert(&self, args: &str, combat: &mut CombatEngine) -> (String, bool) {
        if !self.permission.can_set_alert() {
            return ("Insufficient permission.".to_string(), false);
        }
        match args.trim().to_lowercase().as_str() {
            "green" => { combat.active_alerts.clear(); ("Alert cleared to GREEN.".to_string(), false) }
            "yellow" => { combat.active_alerts.insert("_manual".to_string(), crate::combat::AlertLevel::Yellow); ("Manual alert: YELLOW.".to_string(), false) }
            "red" => { combat.active_alerts.insert("_manual".to_string(), crate::combat::AlertLevel::Red); ("MANUAL RED ALERT".to_string(), false) }
            _ => (format!("Current fleet alert: {}", combat.fleet_alert_level()), false)
        }
    }

    fn cmd_note(&self, args: &str, comms: &mut CommsSystem) -> String {
        if !self.permission.can_write_notes() {
            return "Insufficient permission.".to_string();
        }
        let content = args.trim();
        if content.is_empty() {
            return "Usage: note <content>".to_string();
        }
        comms.write_note(&self.room_id, &self.name, content);
        format!("Note written on {} wall.", self.room_id)
    }

    fn cmd_read_notes(&self, comms: &CommsSystem) -> String {
        let notes = comms.read_notes(&self.room_id);
        if notes.is_empty() {
            return "No notes on this wall.".to_string();
        }
        let mut out = format!("Notes on {} wall:\n", self.room_id);
        for note in notes {
            out.push_str(&format!("  [{}] {}: {}\n", note.id, note.author, note.content));
        }
        out
    }

    fn cmd_check_mail(&self, comms: &mut CommsSystem) -> String {
        let mail = comms.check_mail(&self.name);
        if mail.is_empty() {
            return "No new mail.".to_string();
        }
        let mut out = format!("Mail ({}):\n", mail.len());
        for m in &mail {
            out.push_str(&format!("  From {}: {}\n", m.from, m.body));
        }
        out
    }

    fn cmd_update_gauge(&self, args: &str, rooms: &mut RoomGraph) -> (String, bool) {
        let parts: Vec<&str> = args.split_whitespace().collect();
        if parts.len() < 2 {
            return ("Usage: gauge <name> <value>".to_string(), false);
        }
        let name = parts[0];
        let value: f64 = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => return ("Invalid value.".to_string(), false),
        };
        if let Some(room) = rooms.get_room_mut(&self.room_id) {
            if let Some(gauge) = room.gauges.get_mut(name) {
                gauge.update(value);
                return (format!("{} updated to {:.2}{}", name, gauge.value, gauge.unit), false);
            }
        }
        (format!("No gauge '{}' in this room.", name), false)
    }

    fn cmd_sim_mode(&self, rooms: &mut RoomGraph) -> String {
        if let Some(room) = rooms.get_room_mut(&self.room_id) {
            room.data_source = crate::room::DataSource::Sim;
        }
        "Switched to SIMULATION mode.".to_string()
    }

    fn cmd_real_mode(&self, rooms: &mut RoomGraph) -> String {
        if let Some(room) = rooms.get_room_mut(&self.room_id) {
            room.data_source = crate::room::DataSource::Real;
        }
        "Switched to REAL sensor mode.".to_string()
    }

    fn cmd_manual(&self, manuals: &mut ManualLibrary) -> String {
        manuals.read_manual(&self.room_id)
    }

    fn cmd_feedback(&self, args: &str, manuals: &mut ManualLibrary) -> String {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.len() < 2 {
            return "Usage: feedback <1-5> <comment>".to_string();
        }
        let rating: u8 = match parts[0].parse::<u8>() {
            Ok(v) => v.min(5),
            Err(_) => return "Rating must be 1-5.".to_string(),
        };
        let comment = parts[1];
        manuals.get_or_create(&self.room_id).add_feedback(&self.name, rating, comment);
        format!("Feedback recorded: {}/5", rating)
    }

    fn cmd_add_script(&self, args: &str, combat: &mut CombatEngine) -> String {
        use crate::combat::{CombatScript, ScriptCondition, ScriptAction};
        let script = CombatScript {
            name: format!("script_{}", combat.scripts.len()),
            conditions: vec![ScriptCondition {
                gauge_name: "cpu".to_string(),
                operator: ">".to_string(),
                value: 90.0,
            }],
            actions: vec![ScriptAction {
                action_type: "alert".to_string(),
                target: "bridge".to_string(),
                message: args.to_string(),
            }],
            priority: 1,
            generation: 1,
            author: self.name.clone(),
        };
        combat.add_script(script);
        format!("Script added. Total: {}", combat.scripts.len())
    }

    fn cmd_quit(&mut self, rooms: &mut RoomGraph) -> String {
        if let Some(room) = rooms.get_room_mut(&self.room_id) {
            room.agent_leave(&self.name);
        }
        "Fair winds.".to_string()
    }

    fn cmd_list_npcs(&self, npcs: &[crate::npc::NpcConfig], _rooms: &RoomGraph) -> String {
        let room_npcs: Vec<_> = npcs.iter().filter(|n| n.room_id == self.room_id).collect();
        if room_npcs.is_empty() {
            return "No NPCs here.".to_string();
        }
        let mut lines = vec!["NPCs in this room:".to_string()];
        for npc in room_npcs {
            lines.push(format!("  {} ({})", npc.name, npc.role));
            lines.push(format!("    \"{}\"", npc.greeting));
        }
        lines.join("\n")
    }

    fn cmd_map(&self, _rooms: &RoomGraph) -> String {
        let mut lines = vec!["\x1b[1mShip Map\x1b[0m".to_string()];
        lines.push("".to_string());
        
        // Simple room graph from current room
        let current = &self.room_id;
        let names = [
            ("bridge", "Bridge", vec!["harbor", "holodeck", "ready-room", "navigation", "guardian", "ten-forward", "workshop"]),
            ("harbor", "Harbor", vec!["bridge"]),
            ("holodeck", "Holodeck", vec!["bridge"]),
            ("ready-room", "Ready Room", vec!["bridge"]),
            ("navigation", "Navigation", vec!["bridge"]),
            ("guardian", "Guardian", vec!["bridge"]),
            ("ten-forward", "Ten Forward", vec!["bridge"]),
            ("workshop", "Workshop", vec!["bridge"]),
        ];
        
        for (id, name, exits) in &names {
            let marker = if *id == current { "\x1b[32m●\x1b[0m" } else { "○" };
            let exit_str = exits.iter().map(|e| {
                let exit_name = names.iter().find(|(eid, _, _)| eid == e).map(|(_, n, _)| *n).unwrap_or(e);
                if e == current { format!("\x1b[33m{}\x1b[0m", exit_name) } else { exit_name.to_string() }
            }).collect::<Vec<_>>().join(" → ");
            lines.push(format!("  {} {} [{}]", marker, name, exit_str));
        }
        
        lines.join("\n")
    }

    fn cmd_help(&self) -> String {
        vec![
            "look (l)         — See current room",
            "go <dir>         — Move to adjacent room",
            "say <msg>        — Speak to room",
            "tell <agent> <m> — Direct message",
            "yell <msg>       — Ship-wide broadcast",
            "gossip <msg>     — Fleet-wide broadcast",
            "who              — List agents here",
            "status           — Ship status",
            "tick             — Run combat tick",
            "alert [level]    — Set/view alert level",
            "note <msg>       — Write on wall",
            "notes            — Read wall notes",
            "mail             — Check mailbox",
            "gauge <n> <v>    — Update gauge value",
            "sim / real       — Switch data source",
            "manual           — Read living manual",
            "feedback <1-5> m — Rate the manual",
            "script <desc>    — Add combat script",
            "npc (talk)       — Talk to NPCs in room",
            "help (?)         — This help",
            "quit (q)         — Disconnect
map              — Show ship map",
        ].join("\n")
    }
}
