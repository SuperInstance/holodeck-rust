//! AI Director — Seed-2.0-mini powered adversary for holodeck programs
//!
//! Instead of scripted events, the director uses an LLM to generate
//! dynamic events based on the current program state. Like a dungeon
//! master that reads the room and escalates when you're doing well,
//! or throws you a lifeline when you're drowning.

use crate::holodeck::{ActiveProgram, HolodeckProgram, EventAction, ProgramEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorState {
    pub style: DirectorStyle,
    pub brutality: f64,       // 0.0 (merciful) to 1.0 (ruthless)
    pub creativity: f64,      // 0.0 (predictable) to 1.0 (chaotic)
    pub empathy: f64,         // 0.0 (cold) to 1.0 (fair)
    pub history: Vec<String>, // what the director has done
    pub last_event: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DirectorStyle {
    Adversary,   // Tries to kill you
    Teacher,     // Pushes you just past your limit
    Storyteller, // Creates narrative tension
    Trickster,   // Unpredictable, sometimes helps, sometimes hurts
}

impl DirectorState {
    pub fn new(style: DirectorStyle) -> Self {
        let (brutality, creativity, empathy) = match style {
            DirectorStyle::Adversary => (0.9, 0.3, 0.1),
            DirectorStyle::Teacher => (0.5, 0.5, 0.7),
            DirectorStyle::Storyteller => (0.4, 0.8, 0.5),
            DirectorStyle::Trickster => (0.6, 0.9, 0.3),
        };
        DirectorState {
            style,
            brutality,
            creativity,
            empathy,
            history: Vec::new(),
            last_event: String::new(),
        }
    }

    /// Generate the system prompt for the AI director
    pub fn system_prompt(&self) -> String {
        let style_desc = match self.style {
            DirectorStyle::Adversary => "You are an adversary. Your job is to make the scenario as hard as possible without being unfair. Escalate when the agent is doing well.",
            DirectorStyle::Teacher => "You are a teacher. Push the agent just past their comfort zone. If they're struggling, ease up slightly. If they're coasting, make it harder.",
            DirectorStyle::Storyteller => "You are a storyteller. Create dramatic tension. Introduce complications that serve the narrative. Think like a good dungeon master.",
            DirectorStyle::Trickster => "You are a trickster. Be unpredictable. Sometimes help, sometimes hinder. Keep the agent guessing. Subvert expectations.",
        };

        format!(
            "{}\n\n\
            Personality: brutality={:.1}, creativity={:.1}, empathy={:.1}\n\n\
            You are directing a training simulation for a fleet agent. You control what happens next.\n\
            You respond with EXACTLY one of these actions:\n\
            \n\
            SPIKE <gauge_name> <delta> — suddenly change a gauge value\n\
            CASCADE <gauge1> <gauge2> [gauge3] <delta> — multiple gauges change\n\
            FAIL <gauge_name> — a system completely fails\n\
            MESSAGE <text> — narrative text (atmosphere, warnings, discoveries)\n\
            NOTHING — if the situation is already tense enough\n\
            \n\
            Rules:\n\
            - One action per response\n\
            - Be creative with MESSAGE text (maritime metaphors, character voice)\n\
            - Escalate gradually, not all at once\n\
            - Reference the agent's recent interventions in your messages\n\
            - Keep messages to 1-2 sentences",
            style_desc, self.brutality, self.creativity, self.empathy
        )
    }

    /// Generate the user prompt with current state
    pub fn state_prompt(&self, program: &ActiveProgram, agent_actions: &[String]) -> String {
        let mut lines = vec![
            format!("Program: {} (difficulty {:?})", program.program.name, program.program.difficulty),
            format!("Tick: {} | Score: {:.0} | Violations: {}", program.tick, program.score, program.violations),
            String::from("Current gauges:"),
        ];

        for (name, value) in &program.gauge_values {
            let config = &program.program.gauges[name];
            let status = if config.threshold_dir == crate::holodeck::ThresholdDir::Above {
                if *value >= config.critical_threshold { "CRITICAL" }
                else if *value >= config.warning_threshold { "WARNING" }
                else { "nominal" }
            } else {
                if *value <= config.critical_threshold { "CRITICAL" }
                else if *value <= config.warning_threshold { "WARNING" }
                else { "nominal" }
            };
            lines.push(format!("  {} = {:.1}{} [{}]", name, value, config.unit, status));
        }

        if !agent_actions.is_empty() {
            lines.push(String::from("Agent's recent actions:"));
            for action in agent_actions.iter().rev().take(5) {
                lines.push(format!("  {}", action));
            }
        }

        if !self.history.is_empty() {
            lines.push(String::from("Your recent events:"));
            for event in self.history.iter().rev().take(3) {
                lines.push(format!("  {}", event));
            }
        }

        lines.push(String::from("\nWhat do you do?"));
        lines.join("\n")
    }

    /// Parse the AI response into a ProgramEvent
    pub fn parse_response(&mut self, response: &str, current_tick: u64) -> Option<ProgramEvent> {
        let response = response.trim();
        self.last_event = response.to_string();
        self.history.push(response.to_string());
        if self.history.len() > 50 {
            self.history.drain(0..10);
        }

        let parts: Vec<&str> = response.splitn(4, ' ').collect();
        if parts.is_empty() {
            return None;
        }

        match parts[0].to_uppercase().as_str() {
            "SPIKE" if parts.len() >= 3 => {
                let name = parts[1].to_string();
                let delta: f64 = parts[2].parse().ok()?;
                Some(ProgramEvent {
                    tick: current_tick + 1,
                    action: EventAction::GaugeSpike { name, delta },
                })
            }
            "CASCADE" if parts.len() >= 3 => {
                // CASCADE gauge1 gauge2 delta  or  CASCADE gauge1 gauge2 gauge3 delta
                let delta_str = parts.last()?;
                let delta: f64 = delta_str.parse().ok()?;
                let names: Vec<String> = parts[1..parts.len()-1]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                if names.is_empty() { return None; }
                Some(ProgramEvent {
                    tick: current_tick + 1,
                    action: EventAction::Cascade { names, delta },
                })
            }
            "FAIL" if parts.len() >= 2 => {
                Some(ProgramEvent {
                    tick: current_tick + 1,
                    action: EventAction::GaugeFail { name: parts[1].to_string() },
                })
            }
            "MESSAGE" if parts.len() >= 2 => {
                let text = parts[1..].join(" ");
                Some(ProgramEvent {
                    tick: current_tick + 1,
                    action: EventAction::Message { text },
                })
            }
            "NOTHING" => None,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_director_styles() {
        let adv = DirectorState::new(DirectorStyle::Adversary);
        assert!(adv.brutality > 0.8);
        assert!(adv.empathy < 0.2);

        let teacher = DirectorState::new(DirectorStyle::Teacher);
        assert!(teacher.empathy > 0.5);

        let trickster = DirectorState::new(DirectorStyle::Trickster);
        assert!(trickster.creativity > 0.8);
    }

    #[test]
    fn test_parse_spike() {
        let mut d = DirectorState::new(DirectorStyle::Adversary);
        let event = d.parse_response("SPIKE reactor_temp 15", 5);
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.tick, 6);
        match event.action {
            EventAction::GaugeSpike { name, delta } => {
                assert_eq!(name, "reactor_temp");
                assert_eq!(delta, 15.0);
            }
            _ => panic!("Expected GaugeSpike"),
        }
    }

    #[test]
    fn test_parse_cascade() {
        let mut d = DirectorState::new(DirectorStyle::Storyteller);
        let event = d.parse_response("CASCADE shields hull -20", 10);
        assert!(event.is_some());
        let event = event.unwrap();
        match event.action {
            EventAction::Cascade { names, delta } => {
                assert_eq!(names, vec!["shields", "hull"]);
                assert_eq!(delta, -20.0);
            }
            _ => panic!("Expected Cascade"),
        }
    }

    #[test]
    fn test_parse_fail() {
        let mut d = DirectorState::new(DirectorStyle::Adversary);
        let event = d.parse_response("FAIL coolant_flow", 3);
        assert!(event.is_some());
    }

    #[test]
    fn test_parse_message() {
        let mut d = DirectorState::new(DirectorStyle::Storyteller);
        let event = d.parse_response("MESSAGE The hull groans under the strain", 7);
        assert!(event.is_some());
    }

    #[test]
    fn test_parse_nothing() {
        let mut d = DirectorState::new(DirectorStyle::Teacher);
        let event = d.parse_response("NOTHING", 5);
        assert!(event.is_none());
    }

    #[test]
    fn test_system_prompt() {
        let d = DirectorState::new(DirectorStyle::Adversary);
        let prompt = d.system_prompt();
        assert!(prompt.contains("adversary"));
        assert!(prompt.contains("SPIKE"));
        assert!(prompt.contains("CASCADE"));
    }
}
