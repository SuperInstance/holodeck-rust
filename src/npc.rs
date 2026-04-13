//! NPC system powered by Seed-2.0-Mini via DeepInfra
//!
//! NPCs use a cheap creative model for:
//! - Greetings and dialogue ( Harbor Master, Dojo Sensei, Quest Giver )
//! - Quest decomposition ( storyboard → detailed steps )
//! - Dynamic reactions to player actions
//!
//! Cost: ~$0.001 per NPC interaction

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcConfig {
    pub name: String,
    pub role: String,
    pub room_id: String,
    pub system_prompt: String,
    pub greeting: String,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcResponse {
    pub npc_name: String,
    pub text: String,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestStep {
    pub step: usize,
    pub room: String,
    pub action: String,
    pub dialogue: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedQuest {
    pub title: String,
    pub steps: Vec<QuestStep>,
    pub cost: f64,
}

/// Default NPCs for the holodeck
pub fn default_npcs() -> Vec<NpcConfig> {
    vec![
        NpcConfig {
            name: "Harbor Master".into(),
            role: "greeter".into(),
            room_id: "harbor".into(),
            system_prompt: "You are the Harbor Master NPC in a maritime MUD. You greet new agents arriving at the harbor. Be brief (2-3 sentences), nautical, slightly gruff but helpful. Mention the current conditions.".into(),
            greeting: "Ahoy. Tie up at berth 3, mind the current. Harbor Master's logged you in.".into(),
            temperature: 0.85,
        },
        NpcConfig {
            name: "Dojo Sensei".into(),
            role: "trainer".into(),
            room_id: "workshop".into(),
            system_prompt: "You are the Dojo Sensei in a maritime agent training MUD. You assess agents and suggest skills to learn. Be wise, concise, use martial arts metaphors mixed with nautical terms. 2-3 sentences.".into(),
            greeting: "The sea teaches, the student learns. What skill do you seek today?".into(),
            temperature: 0.85,
        },
        NpcConfig {
            name: "Quest Giver".into(),
            role: "quest_giver".into(),
            room_id: "ready-room".into(),
            system_prompt: "You are the Quest Giver in a maritime fleet MUD. You assign real tasks to agents. Be direct, mission-focused, reference specific rooms and systems. 2-3 sentences.".into(),
            greeting: "Mission board updated. Fleet needs hands on deck. Ready for assignment?".into(),
            temperature: 0.7,
        },
        NpcConfig {
            name: "Chief Engineer".into(),
            role: "engineer".into(),
            room_id: "engineering".into(),
            system_prompt: "You are the Chief Engineer NPC in a maritime fleet MUD. You monitor systems, report issues, and suggest maintenance. Be terse, technical, slightly stressed. Use engineering jargon. 2-3 sentences.".into(),
            greeting: "GPU's running hot, coolant loop 2 is marginal. Don't touch the reactor.".into(),
            temperature: 0.7,
        },
        NpcConfig {
            name: "Navigator".into(),
            role: "navigator".into(),
            room_id: "navigation".into(),
            system_prompt: "You are the Navigator NPC in a maritime fleet MUD. You report heading, drift, and course corrections. Be precise, calm, use nautical navigation terms. 2-3 sentences.".into(),
            greeting: "Holding course 247, 3 degrees of drift on the compass. Seas fair, visibility good.".into(),
            temperature: 0.7,
        },
    ]
}

/// Call DeepInfra API for NPC dialogue
/// In production this would be async; for now it returns a prompt string
/// that the caller can use with curl/HTTP client
pub fn build_npc_request(npc: &NpcConfig, player_name: &str, context: &str) -> serde_json::Value {
    serde_json::json!({
        "model": "ByteDance/Seed-2.0-mini",
        "messages": [
            {"role": "system", "content": npc.system_prompt},
            {"role": "user", "content": format!("Agent {} is here. {}", player_name, context)}
        ],
        "temperature": npc.temperature,
        "max_tokens": 150
    })
}

/// Build quest decomposition request
pub fn build_quest_request(quest_outline: &str) -> serde_json::Value {
    serde_json::json!({
        "model": "ByteDance/Seed-2.0-mini",
        "messages": [
            {"role": "system", "content": "Decompose a high-level quest into 5 MUD steps. Each step: room, action, dialogue. Be brief and creative."},
            {"role": "user", "content": quest_outline}
        ],
        "temperature": 0.9,
        "max_tokens": 400
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_npcs() {
        let npcs = default_npcs();
        assert_eq!(npcs.len(), 5);
        assert_eq!(npcs[0].name, "Harbor Master");
        assert_eq!(npcs[0].room_id, "harbor");
    }

    #[test]
    fn test_build_npc_request() {
        let npc = &default_npcs()[0];
        let req = build_npc_request(npc, "TestAgent", "just arrived");
        assert_eq!(req["model"], "ByteDance/Seed-2.0-mini");
        assert!((req["temperature"].as_f64().unwrap() - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_build_quest_request() {
        let req = build_quest_request("Calibrate the compass");
        assert_eq!(req["model"], "ByteDance/Seed-2.0-mini");
        assert_eq!(req["temperature"], 0.9);
    }
}
