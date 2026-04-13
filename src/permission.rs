//! Permission levels — who can do what in the holodeck.
//! 6 levels from Greenhorn to Architect.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Permission {
    Greenhorn = 0,  // Can look, say, go
    Crew = 1,       // Can use gauges, check mail
    Officer = 2,    // Can write notes, tell, yell
    Captain = 3,    // Can manage rooms, agents, alerts
    Commander = 4,  // Can create/destroy rooms, run combat
    Architect = 5,  // Can modify the ship itself
}

impl Permission {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "greenhorn" => Some(Permission::Greenhorn),
            "crew" => Some(Permission::Crew),
            "officer" => Some(Permission::Officer),
            "captain" => Some(Permission::Captain),
            "commander" => Some(Permission::Commander),
            "architect" => Some(Permission::Architect),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Permission::Greenhorn => "Greenhorn",
            Permission::Crew => "Crew",
            Permission::Officer => "Officer",
            Permission::Captain => "Captain",
            Permission::Commander => "Commander",
            Permission::Architect => "Architect",
        }
    }

    pub fn can_go(&self) -> bool { true }
    pub fn can_look(&self) -> bool { true }
    pub fn can_say(&self) -> bool { true }
    pub fn can_tell(&self) -> bool { true }
    pub fn can_yell(&self) -> bool { *self >= Permission::Crew }
    pub fn can_gossip(&self) -> bool { *self >= Permission::Crew }
    pub fn can_read_gauges(&self) -> bool { *self >= Permission::Crew }
    pub fn can_write_notes(&self) -> bool { *self >= Permission::Crew }
    pub fn can_manage_agents(&self) -> bool { *self >= Permission::Captain }
    pub fn can_set_alert(&self) -> bool { *self >= Permission::Crew }
    pub fn can_create_rooms(&self) -> bool { *self >= Permission::Commander }
    pub fn can_run_combat(&self) -> bool { *self >= Permission::Crew }
    pub fn can_modify_ship(&self) -> bool { *self >= Permission::Architect }
}
