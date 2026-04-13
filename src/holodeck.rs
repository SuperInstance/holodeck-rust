//! Holodeck Programs — configurable scenarios in the holodeck room
//! 
//! Programs are named scenarios that set up gauges, NPCs, and scripts
//! for training, testing, or simulation purposes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolodeckProgram {
    pub name: String,
    pub description: String,
    pub difficulty: Difficulty,
    pub gauges: HashMap<String, GaugeConfig>,
    pub events: Vec<ProgramEvent>,
    pub objective: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Difficulty {
    Cadet,      // Single gauge, slow changes
    Officer,    // Multiple gauges, moderate changes
    Captain,    // Cascading failures, time pressure
    Admiral,    // Everything breaks, survive
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ThresholdDir {
    Above,  // Bad when value >= threshold (e.g., temperature)
    Below,  // Bad when value <= threshold (e.g., coolant, shields, hull)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaugeConfig {
    pub initial: f64,
    pub min: f64,
    pub max: f64,
    pub unit: String,
    pub drift_rate: f64,       // How fast it drifts per tick
    pub noise: f64,            // Random noise amplitude
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub threshold_dir: ThresholdDir,  // Above = bad when high, Below = bad when low
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramEvent {
    pub tick: u64,             // When this event fires
    pub action: EventAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventAction {
    GaugeSpike { name: String, delta: f64 },
    GaugeFail { name: String },
    Message { text: String },
    Cascade { names: Vec<String>, delta: f64 },
}

impl HolodeckProgram {
    /// Built-in programs catalog
    pub fn catalog() -> Vec<Self> {
        vec![
            Self::reactor_training(),
            Self::nav_challenge(),
            Self::cascade_drill(),
            Self::fog_of_war(),
            Self::night_watch(),
        ]
    }

    /// List program names and descriptions
    pub fn list_programs() -> Vec<String> {
        Self::catalog().iter().map(|p| {
            let diff = match p.difficulty {
                Difficulty::Cadet => "CADET",
                Difficulty::Officer => "OFFICER",
                Difficulty::Captain => "CAPTAIN",
                Difficulty::Admiral => "ADMIRAL",
            };
            format!("  {:20} [{}] {}", p.name, diff, p.description)
        }).collect()
    }

    fn reactor_training() -> Self {
        let mut gauges = HashMap::new();
        gauges.insert("reactor_temp".into(), GaugeConfig {
            initial: 65.0, min: 0.0, max: 120.0, unit: "°C".into(),
            drift_rate: 0.5, noise: 0.3, warning_threshold: 85.0, critical_threshold: 100.0,
            threshold_dir: ThresholdDir::Above,
        });
        gauges.insert("coolant_flow".into(), GaugeConfig {
            initial: 100.0, min: 0.0, max: 100.0, unit: "%".into(),
            drift_rate: -0.2, noise: 0.5, warning_threshold: 40.0, critical_threshold: 20.0,
            threshold_dir: ThresholdDir::Below,
        });

        let events = vec![
            ProgramEvent { tick: 10, action: EventAction::GaugeSpike { name: "reactor_temp".into(), delta: 15.0 } },
            ProgramEvent { tick: 20, action: EventAction::GaugeSpike { name: "reactor_temp".into(), delta: 10.0 } },
            ProgramEvent { tick: 25, action: EventAction::Message { text: "⚠ Coolant pump vibration detected".into() } },
            ProgramEvent { tick: 30, action: EventAction::GaugeFail { name: "coolant_flow".into() } },
        ];

        HolodeckProgram {
            name: "reactor-training".into(),
            description: "Keep the reactor from meltdown".into(),
            difficulty: Difficulty::Cadet,
            gauges,
            events,
            objective: "Maintain reactor temp below 100°C for 50 ticks".into(),
        }
    }

    fn nav_challenge() -> Self {
        let mut gauges = HashMap::new();
        gauges.insert("heading".into(), GaugeConfig {
            initial: 0.0, min: 0.0, max: 360.0, unit: "°".into(),
            drift_rate: 2.0, noise: 1.0, warning_threshold: 999.0, critical_threshold: 999.0,
            threshold_dir: ThresholdDir::Above, // never triggers
        });
        gauges.insert("drift".into(), GaugeConfig {
            initial: 0.0, min: -30.0, max: 30.0, unit: "°".into(),
            drift_rate: 0.5, noise: 0.8, warning_threshold: 10.0, critical_threshold: 20.0,
            threshold_dir: ThresholdDir::Above,
        });
        gauges.insert("depth".into(), GaugeConfig {
            initial: 45.0, min: 0.0, max: 200.0, unit: "fm".into(),
            drift_rate: 1.0, noise: 0.5, warning_threshold: 100.0, critical_threshold: 150.0,
            threshold_dir: ThresholdDir::Above,
        });

        let events = vec![
            ProgramEvent { tick: 8, action: EventAction::Message { text: "🌊 Current shift — drift increasing to starboard".into() } },
            ProgramEvent { tick: 15, action: EventAction::Cascade { names: vec!["drift".into(), "depth".into()], delta: 5.0 } },
            ProgramEvent { tick: 25, action: EventAction::Message { text: "⚠ Shoal ahead — depth dropping fast".into() } },
            ProgramEvent { tick: 30, action: EventAction::GaugeSpike { name: "depth".into(), delta: 40.0 } },
        ];

        HolodeckProgram {
            name: "nav-challenge".into(),
            description: "Navigate through shifting currents and shoals".into(),
            difficulty: Difficulty::Officer,
            gauges,
            events,
            objective: "Maintain heading within 10° and avoid grounding for 40 ticks".into(),
        }
    }

    fn cascade_drill() -> Self {
        let mut gauges = HashMap::new();
        gauges.insert("reactor".into(), GaugeConfig {
            initial: 70.0, min: 0.0, max: 120.0, unit: "°C".into(),
            drift_rate: 1.0, noise: 0.5, warning_threshold: 85.0, critical_threshold: 100.0,
            threshold_dir: ThresholdDir::Above,
        });
        gauges.insert("shields".into(), GaugeConfig {
            initial: 100.0, min: 0.0, max: 100.0, unit: "%".into(),
            drift_rate: -0.3, noise: 1.0, warning_threshold: 50.0, critical_threshold: 20.0,
            threshold_dir: ThresholdDir::Below,
        });
        gauges.insert("hull".into(), GaugeConfig {
            initial: 100.0, min: 0.0, max: 100.0, unit: "%".into(),
            drift_rate: 0.0, noise: 0.0, warning_threshold: 70.0, critical_threshold: 40.0,
            threshold_dir: ThresholdDir::Below,
        });
        gauges.insert("life_support".into(), GaugeConfig {
            initial: 100.0, min: 0.0, max: 100.0, unit: "%".into(),
            drift_rate: -0.1, noise: 0.2, warning_threshold: 60.0, critical_threshold: 30.0,
            threshold_dir: ThresholdDir::Below,
        });

        let events = vec![
            ProgramEvent { tick: 5, action: EventAction::GaugeSpike { name: "reactor".into(), delta: 20.0 } },
            ProgramEvent { tick: 10, action: EventAction::GaugeFail { name: "shields".into() } },
            ProgramEvent { tick: 15, action: EventAction::Message { text: "💥 Shield generator offline — hull exposed".into() } },
            ProgramEvent { tick: 20, action: EventAction::Cascade { names: vec!["hull".into(), "life_support".into()], delta: -15.0 } },
            ProgramEvent { tick: 30, action: EventAction::Cascade { names: vec!["reactor".into(), "hull".into(), "life_support".into()], delta: -10.0 } },
        ];

        HolodeckProgram {
            name: "cascade-drill".into(),
            description: "Everything fails at once — triage to survive".into(),
            difficulty: Difficulty::Captain,
            gauges,
            events,
            objective: "Keep at least 2 systems above critical for 40 ticks".into(),
        }
    }

    fn fog_of_war() -> Self {
        let mut gauges = HashMap::new();
        gauges.insert("visibility".into(), GaugeConfig {
            initial: 100.0, min: 0.0, max: 100.0, unit: "%".into(),
            drift_rate: -2.0, noise: 3.0, warning_threshold: 40.0, critical_threshold: 15.0,
            threshold_dir: ThresholdDir::Below,
        });
        gauges.insert("radar".into(), GaugeConfig {
            initial: 100.0, min: 0.0, max: 100.0, unit: "%".into(),
            drift_rate: -1.0, noise: 2.0, warning_threshold: 50.0, critical_threshold: 25.0,
            threshold_dir: ThresholdDir::Below,
        });
        gauges.insert("sonar".into(), GaugeConfig {
            initial: 100.0, min: 0.0, max: 100.0, unit: "%".into(),
            drift_rate: 0.0, noise: 5.0, warning_threshold: 50.0, critical_threshold: 25.0,
            threshold_dir: ThresholdDir::Below,
        });

        let events = vec![
            ProgramEvent { tick: 5, action: EventAction::Message { text: "🌫 Fog banks forming on all quadrants".into() } },
            ProgramEvent { tick: 12, action: EventAction::GaugeSpike { name: "radar".into(), delta: -30.0 } },
            ProgramEvent { tick: 18, action: EventAction::Message { text: "👻 Unidentified contact — bearing 247, range unknown".into() } },
            ProgramEvent { tick: 25, action: EventAction::Cascade { names: vec!["visibility".into(), "radar".into()], delta: -20.0 } },
            ProgramEvent { tick: 35, action: EventAction::Message { text: "⚠ Contact closing — sonar intermittent".into() } },
        ];

        HolodeckProgram {
            name: "fog-of-war".into(),
            description: "Sensors degrading, unknown contact approaching".into(),
            difficulty: Difficulty::Captain,
            gauges,
            events,
            objective: "Track the contact and maintain at least 1 sensor above critical for 40 ticks".into(),
        }
    }

    fn night_watch() -> Self {
        let mut gauges = HashMap::new();
        gauges.insert("watch_alertness".into(), GaugeConfig {
            initial: 100.0, min: 0.0, max: 100.0, unit: "%".into(),
            drift_rate: -0.5, noise: 0.3, warning_threshold: 50.0, critical_threshold: 25.0,
            threshold_dir: ThresholdDir::Below,
        });
        gauges.insert("sea_state".into(), GaugeConfig {
            initial: 2.0, min: 0.0, max: 9.0, unit: "bf".into(),
            drift_rate: 0.2, noise: 0.3, warning_threshold: 5.0, critical_threshold: 7.0,
            threshold_dir: ThresholdDir::Above,
        });
        gauges.insert("traffic_density".into(), GaugeConfig {
            initial: 2.0, min: 0.0, max: 20.0, unit: "vessels".into(),
            drift_rate: 0.3, noise: 0.5, warning_threshold: 8.0, critical_threshold: 15.0,
            threshold_dir: ThresholdDir::Above,
        });

        let events = vec![
            ProgramEvent { tick: 10, action: EventAction::Message { text: "🌙 0300 — the quiet hours. Stay sharp.".into() } },
            ProgramEvent { tick: 18, action: EventAction::GaugeSpike { name: "traffic_density".into(), delta: 5.0 } },
            ProgramEvent { tick: 22, action: EventAction::Message { text: "🚢 Fishing fleet — multiple contacts, AIS clutter".into() } },
            ProgramEvent { tick: 30, action: EventAction::Cascade { names: vec!["sea_state".into(), "traffic_density".into()], delta: 3.0 } },
            ProgramEvent { tick: 35, action: EventAction::Message { text: "⚠ Vessel not responding to hails — bearing 012, closing".into() } },
        ];

        HolodeckProgram {
            name: "night-watch".into(),
            description: "Solo watch, degrading alertness, building traffic".into(),
            difficulty: Difficulty::Admiral,
            gauges,
            events,
            objective: "Maintain alertness and avoid all incidents for 45 ticks".into(),
        }
    }
}

/// Runtime state for an active holodeck program
#[derive(Debug, Clone)]
pub struct ActiveProgram {
    pub program: HolodeckProgram,
    pub tick: u64,
    pub score: f64,
    pub active: bool,
    pub violations: u32,
    pub gauge_values: HashMap<String, f64>,
}

impl ActiveProgram {
    pub fn new(program: HolodeckProgram) -> Self {
        let gauge_values: HashMap<String, f64> = program.gauges.iter()
            .map(|(name, config)| (name.clone(), config.initial))
            .collect();
        ActiveProgram {
            program,
            tick: 0,
            score: 100.0,
            active: true,
            violations: 0,
            gauge_values,
        }
    }

    /// Run one tick of the program simulation
    pub fn tick(&mut self) -> Vec<String> {
        if !self.active { return vec!["Program ended.".into()]; }
        
        self.tick += 1;
        let mut messages = Vec::new();
        
        // Apply drift and noise to gauges
        for (name, config) in &self.program.gauges {
            let current = self.gauge_values.get(name).copied().unwrap_or(config.initial);
            let drift = config.drift_rate + (rand_simple() * 2.0 - 1.0) * config.noise;
            let new_val = (current + drift).clamp(config.min, config.max);
            self.gauge_values.insert(name.clone(), new_val);
            
            // Check thresholds — direction depends on drift
            // If drift_rate is negative, gauge is failing when LOW (below threshold = bad)
            // If drift_rate is positive, gauge is failing when HIGH (above threshold = bad)
            let is_critical = match config.threshold_dir {
                ThresholdDir::Above => new_val >= config.critical_threshold,
                ThresholdDir::Below => new_val <= config.critical_threshold,
            };
            let is_warning = !is_critical && match config.threshold_dir {
                ThresholdDir::Above => new_val >= config.warning_threshold,
                ThresholdDir::Below => new_val <= config.warning_threshold,
            };
            if is_critical {
                self.violations += 1;
                self.score -= 2.0;
                messages.push(format!("🔴 CRITICAL: {} = {:.1}{}", name, new_val, config.unit));
            } else if is_warning {
                self.score -= 0.5;
                messages.push(format!("🟡 WARNING: {} = {:.1}{}", name, new_val, config.unit));
            }
        }
        
        // Fire scheduled events
        for event in &self.program.events {
            if event.tick == self.tick {
                match &event.action {
                    EventAction::Message { text } => messages.push(text.clone()),
                    EventAction::GaugeSpike { name, delta } => {
                        if let Some(val) = self.gauge_values.get_mut(name) {
                            *val = (*val + delta).clamp(
                                self.program.gauges[name].min,
                                self.program.gauges[name].max,
                            );
                            messages.push(format!("⚡ {}: {:+.1}", name, delta));
                        }
                    }
                    EventAction::GaugeFail { name } => {
                        if let Some(val) = self.gauge_values.get_mut(name) {
                            *val = 0.0;
                            messages.push(format!("💀 {}: FAILED", name));
                        }
                    }
                    EventAction::Cascade { names, delta } => {
                        for n in names {
                            if let Some(val) = self.gauge_values.get_mut(n) {
                                *val = (*val + delta).clamp(
                                    self.program.gauges.get(n).map(|g| g.min).unwrap_or(0.0),
                                    self.program.gauges.get(n).map(|g| g.max).unwrap_or(100.0),
                                );
                            }
                        }
                        messages.push(format!("🌊 CASCADE: {} systems affected", names.len()));
                    }
                }
            }
        }
        
        // End conditions
        if self.score <= 0.0 {
            self.active = false;
            messages.push("💀 PROGRAM FAILED — score reached zero".into());
        } else if self.tick >= 50 {
            self.active = false;
            messages.push(format!("✅ PROGRAM COMPLETE — score: {:.0}", self.score));
        }
        
        if messages.is_empty() {
            messages.push(format!("Tick {} — all systems nominal. Score: {:.0}", self.tick, self.score));
        }
        
        messages
    }

    /// Agent adjusts a gauge (intervention)
    pub fn adjust(&mut self, gauge_name: &str, delta: f64) -> String {
        if !self.active { return "No program running.".into(); }
        match self.gauge_values.get_mut(gauge_name) {
            Some(val) => {
                let config = &self.program.gauges[gauge_name];
                *val = (*val + delta).clamp(config.min, config.max);
                self.score -= 0.2; // intervention costs points
                format!("Adjusted {} by {:+.1} → {:.1}{} (score: {:.0})", 
                    gauge_name, delta, *val, config.unit, self.score)
            }
            None => format!("No gauge '{}' in this program. Gauges: {}", 
                gauge_name, self.gauge_values.keys().cloned().collect::<Vec<_>>().join(", "))
        }
    }

    pub fn status(&self) -> String {
        let mut lines = vec![
            format!("📋 Program: {} (difficulty {:?})", self.program.name, self.program.difficulty),
            format!("🎯 Objective: {}", self.program.objective),
            format!("⏱ Tick: {}/50 | Score: {:.0} | Violations: {}", self.tick, self.score, self.violations),
            String::new(),
        ];
        for (name, value) in &self.gauge_values {
            let config = &self.program.gauges[name];
            let indicator = {
                let config = &self.program.gauges[name];
                let is_critical = match config.threshold_dir {
                    ThresholdDir::Above => *value >= config.critical_threshold,
                    ThresholdDir::Below => *value <= config.critical_threshold,
                };
                let is_warning = !is_critical && match config.threshold_dir {
                    ThresholdDir::Above => *value >= config.warning_threshold,
                    ThresholdDir::Below => *value <= config.warning_threshold,
                };
                if is_critical { "🔴" } else if is_warning { "🟡" } else { "🟢" }
            };
            lines.push(format!("  {} {}: {:.1}{}", indicator, name, value, config.unit));
        }
        lines.join("\n")
    }
}

/// Simple deterministic-ish random for simulation
fn rand_simple() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    // Use low bits for pseudo-randomness
    (nanos as f64 / 1_000_000_000.0).fract()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog() {
        let programs = HolodeckProgram::catalog();
        assert_eq!(programs.len(), 5);
        assert!(programs.iter().any(|p| p.name == "reactor-training"));
        assert!(programs.iter().any(|p| p.name == "night-watch"));
    }

    #[test]
    fn test_list_programs() {
        let list = HolodeckProgram::list_programs();
        assert_eq!(list.len(), 5);
        assert!(list[0].contains("CADET"));
    }

    #[test]
    fn test_program_run() {
        let program = HolodeckProgram::reactor_training();
        let mut active = ActiveProgram::new(program);
        
        // Run a few ticks
        for _ in 0..5 {
            let msgs = active.tick();
            assert!(!msgs.is_empty());
        }
        assert_eq!(active.tick, 5);
        assert!(active.score <= 100.0);
    }

    #[test]
    fn test_adjust_gauge() {
        let program = HolodeckProgram::reactor_training();
        let mut active = ActiveProgram::new(program);
        
        let result = active.adjust("reactor_temp", -10.0);
        assert!(result.contains("reactor_temp"));
        assert!(result.contains("-10"));
    }

    #[test]
    fn test_invalid_adjust() {
        let program = HolodeckProgram::reactor_training();
        let mut active = ActiveProgram::new(program);
        
        let result = active.adjust("nonexistent", 1.0);
        assert!(result.contains("No gauge"));
    }

    #[test]
    fn test_program_completes() {
        let program = HolodeckProgram::reactor_training();
        let mut active = ActiveProgram::new(program);
        
        for _ in 0..55 {
            active.tick();
        }
        assert!(!active.active);
    }
}
