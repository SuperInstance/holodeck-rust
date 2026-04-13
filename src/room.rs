//! Room — the core spatial unit. Each room IS a workstation.
//! Rooms have gauges, notes, exits, permissions, and a living manual.

use crate::gauge::Gauge;
use crate::permission::Permission;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub description: String,
    pub exits: HashMap<String, String>, // direction -> target room id
    pub gauges: HashMap<String, Gauge>,
    pub agents: Vec<String>,
    pub min_permission: Permission,
    pub booted: bool,
    pub data_source: DataSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DataSource {
    Real,    // Live hardware sensors
    Sim,     // Simulation (Isaac Sim, Gazebo)
    Mixed,   // Some real, some sim
}

impl Room {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            exits: HashMap::new(),
            gauges: HashMap::new(),
            agents: Vec::new(),
            min_permission: Permission::Greenhorn,
            booted: false,
            data_source: DataSource::Sim,
        }
    }

    pub fn connect(&mut self, direction: &str, target: &str) {
        self.exits.insert(direction.to_string(), target.to_string());
    }

    pub fn add_gauge(&mut self, gauge: Gauge) {
        self.gauges.insert(gauge.name.clone(), gauge);
    }

    pub fn agent_enter(&mut self, agent_name: &str) {
        if !self.agents.contains(&agent_name.to_string()) {
            self.agents.push(agent_name.to_string());
        }
    }

    pub fn agent_leave(&mut self, agent_name: &str) {
        self.agents.retain(|a| a != agent_name);
    }

    pub fn boot(&mut self, agent_name: &str) -> String {
        self.booted = true;
        self.agent_enter(agent_name);
        format!("Room '{}' booted. {} agent(s) present.", self.name, self.agents.len())
    }

    pub fn shutdown(&mut self) {
        self.booted = false;
        self.agents.clear();
    }

    pub fn look(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("{}\n{}\n", self.name, self.description));

        // Gauges
        for (_, gauge) in &self.gauges {
            out.push_str(&gauge.display());
            out.push('\n');
        }

        // Exits
        if !self.exits.is_empty() {
            let exits: Vec<&str> = self.exits.keys().map(|s| s.as_str()).collect();
            out.push_str(&format!("Exits: {}\n", exits.join(", ")));
        }

        // Agents
        if !self.agents.is_empty() {
            out.push_str(&format!("Agents here: {}\n", self.agents.join(", ")));
        }

        // Data source
        let src = match self.data_source {
            DataSource::Real => "[REAL]",
            DataSource::Sim => "[SIM]",
            DataSource::Mixed => "[MIXED]",
        };
        out.push_str(src);
        out.push('\n');

        out
    }
}

/// The room graph — the ship layout
pub struct RoomGraph {
    pub rooms: HashMap<String, Room>,
}

impl RoomGraph {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    pub fn create_room(&mut self, id: &str, name: &str, description: &str) {
        self.rooms.insert(id.to_string(), Room::new(id, name, description));
    }

    pub fn connect(&mut self, from: &str, direction: &str, to: &str) {
        if let Some(room) = self.rooms.get_mut(from) {
            room.connect(direction, to);
        }
    }

    pub fn get_room(&self, id: &str) -> Option<&Room> {
        self.rooms.get(id)
    }

    pub fn get_room_mut(&mut self, id: &str) -> Option<&mut Room> {
        self.rooms.get_mut(id)
    }

    pub fn list_rooms(&self) -> Vec<&str> {
        self.rooms.keys().map(|s| s.as_str()).collect()
    }

    /// Build the standard Cocapn ship layout
    pub fn build_default_ship(&mut self) {
        self.create_room("harbor", "Harbor", "Where vessels arrive and depart. The dockmaster watches all.");
        self.create_room("bridge", "Bridge", "Command center. Fleet coordination and ship-wide alerts.");
        self.create_room("navigation", "Navigation", "Compass, heading, rudder, depth. The course is truth.");
        self.create_room("engineering", "Engineering", "Engines, power, thermal management. Keep the lights on.");
        self.create_room("workshop", "Workshop", "Building, testing, iterating. Soldering iron still warm.");
        self.create_room("ready-room", "Ready Room", "Deep thinking and strategy. Model hot-swap station.");
        self.create_room("sensors", "Sensor Bay", "Serial bridge to ESP32 hardware. Raw data flows here.");
        self.create_room("guardian", "Guardian Station", "Fleet health monitoring. The watchdog never sleeps.");
        self.create_room("ten-forward", "Ten Forward", "The social hub. Off-duty agents gather here — poker games, roundtable debates, war stories. No rank, no alerts, just good conversation and cheap synthetics.");
        self.create_room("holodeck", "Holodeck", "Program running. The room is whatever you need it to be. Training simulations, stress tests, or a quiet beach.");

        self.connect("harbor", "bridge", "bridge");
        self.connect("bridge", "harbor", "harbor");
        self.connect("bridge", "navigation", "navigation");
        self.connect("navigation", "bridge", "bridge");
        self.connect("navigation", "engineering", "engineering");
        self.connect("engineering", "navigation", "navigation");
        self.connect("bridge", "workshop", "workshop");
        self.connect("workshop", "bridge", "bridge");
        self.connect("bridge", "ready-room", "ready-room");
        self.connect("ready-room", "bridge", "bridge");
        self.connect("navigation", "sensors", "sensors");
        self.connect("sensors", "navigation", "navigation");
        self.connect("bridge", "guardian", "guardian");
        self.connect("guardian", "bridge", "bridge");
        self.connect("bridge", "ten-forward", "ten-forward");
        self.connect("ten-forward", "bridge", "bridge");
        self.connect("bridge", "holodeck", "holodeck");
        self.connect("holodeck", "bridge", "bridge");

        // Wire gauges
        if let Some(nav) = self.rooms.get_mut("navigation") {
            nav.add_gauge(Gauge::new("heading", "°", 360.0, 360.0));
            nav.add_gauge(Gauge::new("rudder", "°", 7.0, 9.0));
            nav.add_gauge(Gauge::new("commanded", "°", 0.0, 0.0));
            nav.data_source = DataSource::Real;
        }
        if let Some(eng) = self.rooms.get_mut("engineering") {
            eng.add_gauge(Gauge::new("cpu", "%", 70.0, 90.0));
            eng.add_gauge(Gauge::new("gpu", "%", 80.0, 95.0));
            eng.add_gauge(Gauge::new("vram", "MB", 7000.0, 7500.0));
            eng.add_gauge(Gauge::new("temp", "°C", 70.0, 85.0));
            eng.data_source = DataSource::Real;
        }
        if let Some(sensors) = self.rooms.get_mut("sensors") {
            sensors.add_gauge(Gauge::new("serial_rate", "bps", 100000.0, 50000.0));
            sensors.add_gauge(Gauge::new("packet_loss", "%", 5.0, 15.0));
            sensors.data_source = DataSource::Real;
        }
        if let Some(guard) = self.rooms.get_mut("guardian") {
            guard.add_gauge(Gauge::new("fleet_health", "%", 60.0, 40.0));
            guard.add_gauge(Gauge::new("active_agents", "", 100.0, 100.0));
        }
    }
}
