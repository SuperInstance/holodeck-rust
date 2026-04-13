//! Combat — the monitoring tick system.
//! Combat ticks are periodic evaluations of room gauges.
//! Scripts evolve through human demonstration.

use crate::gauge::{Gauge, GaugeStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatScript {
    pub name: String,
    pub conditions: Vec<ScriptCondition>,
    pub actions: Vec<ScriptAction>,
    pub priority: u32,
    pub generation: u32,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptCondition {
    pub gauge_name: String,
    pub operator: String, // ">", "<", "==", "jitter>", "trend>"
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptAction {
    pub action_type: String, // "alert", "log", "command", "notify"
    pub target: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatTick {
    pub tick_number: u64,
    pub timestamp: i64,
    pub room_id: String,
    pub alerts: Vec<Alert>,
    pub scripts_fired: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub level: AlertLevel,
    pub room_id: String,
    pub gauge_name: String,
    pub value: f64,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    Green,
    Yellow,
    Red,
}

impl std::fmt::Display for AlertLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AlertLevel::Green => write!(f, "GREEN"),
            AlertLevel::Yellow => write!(f, "YELLOW"),
            AlertLevel::Red => write!(f, "RED"),
        }
    }
}

pub struct CombatEngine {
    pub scripts: Vec<CombatScript>,
    pub tick_count: u64,
    pub active_alerts: HashMap<String, AlertLevel>, // room_id -> max alert
    pub history: Vec<CombatTick>,
}

impl CombatEngine {
    pub fn new() -> Self {
        Self {
            scripts: Vec::new(),
            tick_count: 0,
            active_alerts: HashMap::new(),
            history: Vec::new(),
        }
    }

    /// Run one combat tick for a room's gauges
    pub fn tick(&mut self, room_id: &str, gauges: &HashMap<String, Gauge>) -> CombatTick {
        self.tick_count += 1;
        let mut tick = CombatTick {
            tick_number: self.tick_count,
            timestamp: chrono::Utc::now().timestamp(),
            room_id: room_id.to_string(),
            alerts: Vec::new(),
            scripts_fired: Vec::new(),
        };

        // Check each gauge for threshold breaches
        let mut max_alert = AlertLevel::Green;
        for gauge in gauges.values() {
            let alert = match gauge.status() {
                GaugeStatus::Red => {
                    tick.alerts.push(Alert {
                        level: AlertLevel::Red,
                        room_id: room_id.to_string(),
                        gauge_name: gauge.name.clone(),
                        value: gauge.value,
                        message: format!("{} CRITICAL: {} = {:.2}{}", room_id, gauge.name, gauge.value, gauge.unit),
                    });
                    AlertLevel::Red
                }
                GaugeStatus::Yellow => {
                    tick.alerts.push(Alert {
                        level: AlertLevel::Yellow,
                        room_id: room_id.to_string(),
                        gauge_name: gauge.name.clone(),
                        value: gauge.value,
                        message: format!("{} WARNING: {} = {:.2}{}", room_id, gauge.name, gauge.value, gauge.unit),
                    });
                    AlertLevel::Yellow
                }
                GaugeStatus::Green => AlertLevel::Green,
            };
            if alert > max_alert {
                max_alert = alert;
            }
        }

        // Evaluate scripts
        for script in &self.scripts {
            if self.evaluate_script(script, gauges) {
                tick.scripts_fired.push(script.name.clone());
            }
        }

        // Update room alert level
        self.active_alerts.insert(room_id.to_string(), max_alert);

        // Keep last 1000 ticks
        self.history.push(tick.clone());
        if self.history.len() > 1000 {
            self.history.remove(0);
        }

        tick
    }

    fn evaluate_script(&self, script: &CombatScript, gauges: &HashMap<String, Gauge>) -> bool {
        for cond in &script.conditions {
            let gauge = match gauges.get(&cond.gauge_name) {
                Some(g) => g,
                None => return false,
            };
            let val = match cond.operator.as_str() {
                ">" => gauge.value > cond.value,
                "<" => gauge.value < cond.value,
                "==" => (gauge.value - cond.value).abs() < 0.01,
                "jitter>" => gauge.jitter() > cond.value,
                "trend>" => gauge.trend() > cond.value,
                _ => false,
            };
            if !val {
                return false;
            }
        }
        true
    }

    /// Add a combat script (from human demonstration or agent evolution)
    pub fn add_script(&mut self, script: CombatScript) {
        self.scripts.push(script);
        // Sort by priority (highest first)
        self.scripts.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Fleet-wide alert level
    pub fn fleet_alert_level(&self) -> AlertLevel {
        self.active_alerts.values().copied().max().unwrap_or(AlertLevel::Green)
    }
}

impl PartialOrd for AlertLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AlertLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let v = |l: &AlertLevel| match l {
            AlertLevel::Green => 0,
            AlertLevel::Yellow => 1,
            AlertLevel::Red => 2,
        };
        v(self).cmp(&v(other))
    }
}
