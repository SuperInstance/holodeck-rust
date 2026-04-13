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
            permission: Permission::Crew, // Default for new agents
            energy: 1000,
            connected_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn handle_command(
        &mut self,
        input: &str,
        rooms: &mut RoomGraph,
        comms: &mut CommsSystem,
        combat: &mut CombatEngine,
        manuals: &mut ManualLibrary,
    ) -> (String, bool) {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let args = parts.get(1).unwrap_or(&"");

        match cmd.as_str() {
            "look" | "l" => (self.cmd_look(rooms), false),
            "go" | "move" | "walk" => self.cmd_go(args, rooms),
            "say" | "\"" => self.cmd_say(args, comms),
            "tell" => self.cmd_tell(args, comms),
            "yell" | "broadcast" => self.cmd_yell(args, comms),
            "gossip" => self.cmd_gossip(args, comms),
            "who" => ("__WHO_LIST__".to_string(), false),
            "status" => (self.cmd_status(rooms, combat), false),
            "tick" => self.cmd_tick(rooms, combat),
            "alert" => self.cmd_alert(args, combat),
            "note" => self.cmd_note(args, rooms, comms),
            "notes" => self.cmd_read_notes(rooms, comms),
            "mail" | "mailbox" => self.cmd_check_mail(comms),
            "gauge" => self.cmd_update_gauge(args, rooms),
            "sim" => self.cmd_sim_mode(rooms),
            "real" => self.cmd_real_mode(rooms),
            "manual" => (self.cmd_manual(rooms, manuals), false),
            "feedback" => self.cmd_feedback(args, rooms, manuals),
            "script" => self.cmd_add_script(args, combat),
            "permission" | "perms" => (format!("Your permission level: {}", self.permission.name()), false),
            "help" | "?" => (self.cmd_help(), false),
            "quit" | "exit" | "q" => (self.cmd_quit(rooms), true),
            "" => ("> ".to_string(), false),
            _ => (format!("Unknown command: {}. Type 'help' for commands.", cmd), false),
        }
    }

    fn cmd_look(&self, rooms: &RoomGraph) -> String {
        match rooms.get_room(&self.room_id) {
            Some(room) => room.look(),
            None => "You are nowhere.".to_string(),
        }
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
                // Leave current room
                if let Some(room) = rooms.get_room_mut(&self.room_id) {
                    room.agent_leave(&self.name);
                }
                // Enter new room
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

    fn cmd_say(&self, args: &str, comms: &mut CommsSystem) -> (String, bool) {
        let msg = comms.say(&self.name, &self.room_id, args);
        (format!("You say: {}", msg.content), false)
    }

    fn cmd_tell(&self, args: &str, comms: &mut CommsSystem) -> (String, bool) {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.len() < 2 {
            return ("Usage: tell <agent> <message>".to_string(), false);
        }
        let target = parts[0];
        let message = parts[1];
        comms.tell(&self.name, target, message);
        (format!("You tell {}: {}", target, message), false)
    }

    fn cmd_yell(&self, args: &str, comms: &mut CommsSystem) -> (String, bool) {
        if !self.permission.can_yell() {
            return ("Insufficient permission to yell.".to_string(), false);
        }
        comms.yell(&self.name, args);
        (format!("[BRIDGE] {} yells: {}", self.name, args), false)
    }

    fn cmd_gossip(&self, args: &str, comms: &mut CommsSystem) -> (String, bool) {
        if !self.permission.can_gossip() {
            return ("Insufficient permission to gossip.".to_string(), false);
        }
        comms.gossip(&self.name, args);
        (format!("[FLEET] {} gossips: {}", self.name, args), false)
    }

    pub fn cmd_who(&self, _agents: &std::collections::HashMap<String, Agent>, rooms: &RoomGraph) -> String {
        let mut out = String::from("Agents online:\n");
        for (_name, _agent) in _agents {
            let room_name = rooms.get_room(&_agent.room_id)
                .map(|r| r.name.as_str())
                .unwrap_or("???");
            out.push_str(&format!("  {} [{}] @ {} ({})\n",
                _agent.name, _agent.permission.name(), room_name, _agent.room_id));
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
                let alert_count = tick.alerts.len();
                results.push(format!("  {}: {} alerts, {} scripts fired", room_id, alert_count, tick.scripts_fired.len()));
            }
        }
        let fleet_alert = combat.fleet_alert_level();
        (format!("Combat Tick {}:\n{}\nFleet Alert: {}", combat.tick_count, results.join("\n"), fleet_alert), false)
    }

    fn cmd_alert(&self, args: &str, combat: &mut CombatEngine) -> (String, bool) {
        if !self.permission.can_set_alert() {
            return ("Insufficient permission.".to_string(), false);
        }
        let level = args.trim().to_lowercase();
        match level.as_str() {
            "green" => { combat.active_alerts.clear(); ("Alert cleared to GREEN.".to_string(), false) }
            "yellow" => { combat.active_alerts.insert("_manual".to_string(), crate::combat::AlertLevel::Yellow); ("Manual alert: YELLOW.".to_string(), false) }
            "red" => { combat.active_alerts.insert("_manual".to_string(), crate::combat::AlertLevel::Red); ("⚠ MANUAL RED ALERT ⚠".to_string(), false) }
            _ => (format!("Current fleet alert: {}", combat.fleet_alert_level()), false)
        }
    }

    fn cmd_note(&self, args: &str, _rooms: &RoomGraph, comms: &mut CommsSystem) -> (String, bool) {
        if !self.permission.can_write_notes() {
            return ("Insufficient permission.".to_string(), false);
        }
        let content = args.trim();
        if content.is_empty() {
            return ("Usage: note <content>".to_string(), false);
        }
        comms.write_note(&self.room_id, &self.name, content);
        (format!("Note written on {} wall.", self.room_id), false)
    }

    fn cmd_read_notes(&self, _rooms: &RoomGraph, comms: &CommsSystem) -> (String, bool) {
        let notes = comms.read_notes(&self.room_id);
        if notes.is_empty() {
            return ("No notes on this wall.".to_string(), false);
        }
        let mut out = format!("Notes on {} wall:\n", self.room_id);
        for note in notes {
            out.push_str(&format!("  [{}] {}: {}\n", note.id, note.author, note.content));
        }
        (out, false)
    }

    fn cmd_check_mail(&self, comms: &mut CommsSystem) -> (String, bool) {
        let mail = comms.check_mail(&self.name);
        if mail.is_empty() {
            return ("No new mail.".to_string(), false);
        }
        let mut out = format!("Mail ({}):\n", mail.len());
        for m in &mail {
            out.push_str(&format!("  From {}: {}\n", m.from, m.body));
        }
        (out, false)
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

    fn cmd_sim_mode(&self, rooms: &mut RoomGraph) -> (String, bool) {
        if let Some(room) = rooms.get_room_mut(&self.room_id) {
            room.data_source = crate::room::DataSource::Sim;
        }
        ("Switched to SIMULATION mode.".to_string(), false)
    }

    fn cmd_real_mode(&self, rooms: &mut RoomGraph) -> (String, bool) {
        if let Some(room) = rooms.get_room_mut(&self.room_id) {
            room.data_source = crate::room::DataSource::Real;
        }
        ("Switched to REAL sensor mode.".to_string(), false)
    }

    fn cmd_manual(&self, _rooms: &RoomGraph, manuals: &mut ManualLibrary) -> String {
        manuals.read_manual(&self.room_id)
    }

    fn cmd_feedback(&self, args: &str, _rooms: &RoomGraph, manuals: &mut ManualLibrary) -> (String, bool) {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.len() < 2 {
            return ("Usage: feedback <1-5> <comment>".to_string(), false);
        }
        let rating: u8 = match parts[0].parse() {
            Ok(v) => v,
            Err(_) => return ("Rating must be 1-5.".to_string(), false),
        };
        let comment = parts[1];
        manuals.get_or_create(&self.room_id).add_feedback(&self.name, rating, comment);
        (format!("Feedback recorded: {}/5", rating.min(5)), false)
    }

    fn cmd_add_script(&self, args: &str, combat: &mut CombatEngine) -> (String, bool) {
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
        (format!("Script added. Total: {}", combat.scripts.len()), false)
    }

    fn cmd_quit(&mut self, rooms: &mut RoomGraph) -> String {
        if let Some(room) = rooms.get_room_mut(&self.room_id) {
            room.agent_leave(&self.name);
        }
        "Fair winds.".to_string()
    }

    fn cmd_help(&self) -> String {
        let mut cmds = vec![
            "look (l)         — See current room",
            "go <dir>         — Move to adjacent room",
            "say <msg>        — Speak to room",
            "tell <agent> <m> — Direct message",
            "yell <msg>       — Ship-wide broadcast",
            "gossip <msg>     — Fleet-wide broadcast",
            "who              — List online agents",
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
            "help (?)         — This help",
            "quit (q)         — Disconnect",
        ];
        if self.permission >= Permission::Commander {
            cmds.push("--- Commander+ ---");
        }
        cmds.join("\n")
    }
}
