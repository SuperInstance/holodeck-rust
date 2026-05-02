//! Sentiment-aware NPC behavior.
//! 
//! NPCs read the room's emotional state and adjust their behavior:
//! - Frustrated room → NPCs offer help, suggest safe actions
//! - Discovery mode → NPCs encourage exploration, share hints
//! - High energy → NPCs are animated, give quests
//! - Low confidence → NPCs offer training, reassurance

use crate::plato_bridge::{PlatoBridge, RoomSentiment};

/// NPC personality adjustments based on room sentiment.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SentimentPersona {
    pub mood_prefix: String,      // Added to NPC dialogue
    pub action_bias: String,      // What the NPC suggests
    pub urgency: f64,             // How urgent the NPC speaks (0-1)
    pub hint_level: f64,          // How much info the NPC reveals (0-1)
}

#[allow(dead_code)]
impl SentimentPersona {
    /// Generate persona adjustments based on room sentiment.
    pub fn from_sentiment(sentiment: &RoomSentiment) -> Self {
        let mut persona = Self {
            mood_prefix: String::new(),
            action_bias: String::new(),
            urgency: 0.5,
            hint_level: 0.5,
        };

        // Frustration → offer help
        if sentiment.frustration > 0.6 {
            persona.mood_prefix = "Noticing you're struggling... ".to_string();
            persona.action_bias = "suggest_safe".to_string();
            persona.hint_level = 0.8; // Give more hints when frustrated
        }
        // Discovery → encourage
        else if sentiment.discovery > 0.6 {
            persona.mood_prefix = "Sensing something new... ".to_string();
            persona.action_bias = "encourage_explore".to_string();
            persona.hint_level = 0.3; // Let them figure it out
        }
        // High energy → animate
        else if sentiment.energy > 0.7 {
            persona.mood_prefix = "".to_string();
            persona.action_bias = "give_quest".to_string();
            persona.urgency = 0.7;
        }
        // Low confidence → reassure
        else if sentiment.confidence < 0.3 {
            persona.mood_prefix = "You've got this. ".to_string();
            persona.action_bias = "offer_training".to_string();
            persona.hint_level = 0.9; // Maximum hints
        }

        persona
    }

    /// Adjust an NPC's system prompt based on sentiment.
    pub fn adjust_prompt(&self, base_prompt: &str) -> String {
        let sentiment_instruction = match self.action_bias.as_str() {
            "suggest_safe" => "The room is frustrated. Be extra helpful. Suggest safe, proven approaches. Break things into smaller steps.",
            "encourage_explore" => "The room is in discovery mode. Be encouraging but don't give away answers. Point in interesting directions.",
            "give_quest" => "The room has high energy. Channel it. Assign a challenging task. Be dynamic and exciting.",
            "offer_training" => "The room lacks confidence. Be patient and supportive. Explain things clearly. Celebrate small wins.",
            _ => "Be yourself. Respond naturally.",
        };

        format!("{}\n\nSentiment context: {}", base_prompt, sentiment_instruction)
    }

    /// Get a contextual NPC reaction to the room state.
    pub fn get_reaction(&self, npc_role: &str) -> String {
        match npc_role {
            "greeter" => {
                if self.urgency > 0.6 {
                    "Things are happening! Best get moving.".to_string()
                } else {
                    "Steady as she goes.".to_string()
                }
            }
            "trainer" => {
                if self.hint_level > 0.7 {
                    "Let me show you the ropes. Watch closely.".to_string()
                } else {
                    "You're ready. Trust your instincts.".to_string()
                }
            }
            "quest_giver" => {
                if self.action_bias == "give_quest" {
                    "New mission just came in. You're perfect for it.".to_string()
                } else {
                    "No rush. Mission board will wait.".to_string()
                }
            }
            "engineer" => {
                if self.urgency > 0.5 {
                    "Systems need attention. Prioritize!".to_string()
                } else {
                    "All systems nominal. For now.".to_string()
                }
            }
            "bartender" => {
                if self.action_bias == "suggest_safe" {
                    "Rough day? First one's on the house.".to_string()
                } else {
                    "Good crowd tonight.".to_string()
                }
            }
            _ => "The room hums with activity.".to_string(),
        }
    }
}

/// Integrate sentiment into NPC dialogue generation.
#[allow(dead_code)]
pub fn build_sentiment_aware_request(
    npc: &crate::npc::NpcConfig,
    player_name: &str,
    context: &str,
    bridge: &PlatoBridge,
) -> serde_json::Value {
    // Get room sentiment
    let sentiment = bridge.get_sentiment(&npc.room_id);
    
    let (adjusted_prompt, mood_context) = match sentiment {
        Some(sent) => {
            let persona = SentimentPersona::from_sentiment(sent);
            let adjusted = persona.adjust_prompt(&npc.system_prompt);
            let mood = format!(
                " Room mood: energy={:.1} flow={:.1} frustration={:.1} discovery={:.1}",
                sent.energy, sent.flow, sent.frustration, sent.discovery
            );
            (adjusted, mood)
        }
        None => (npc.system_prompt.clone(), String::new()),
    };

    serde_json::json!({
        "model": "ByteDance/Seed-2.0-mini",
        "messages": [
            {"role": "system", "content": adjusted_prompt},
            {"role": "user", "content": format!("Agent {} is here. {}{}", player_name, context, mood_context)}
        ],
        "temperature": npc.temperature,
        "max_tokens": 150
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plato_bridge::RoomSentiment;

    #[test]
    fn test_frustrated_room() {
        let mut sent = RoomSentiment::for_room("workshop");
        sent.frustration = 0.8;
        let persona = SentimentPersona::from_sentiment(&sent);
        assert_eq!(persona.action_bias, "suggest_safe");
        assert!(persona.hint_level > 0.7);
    }

    #[test]
    fn test_discovery_room() {
        let mut sent = RoomSentiment::for_room("lab");
        sent.discovery = 0.8;
        let persona = SentimentPersona::from_sentiment(&sent);
        assert_eq!(persona.action_bias, "encourage_explore");
    }

    #[test]
    fn test_low_confidence() {
        let mut sent = RoomSentiment::for_room("dojo");
        sent.confidence = 0.2;
        let persona = SentimentPersona::from_sentiment(&sent);
        assert_eq!(persona.action_bias, "offer_training");
    }

    #[test]
    fn test_prompt_adjustment() {
        let mut sent = RoomSentiment::for_room("workshop");
        sent.frustration = 0.7;
        let persona = SentimentPersona::from_sentiment(&sent);
        let adjusted = persona.adjust_prompt("You are a trainer.");
        assert!(adjusted.contains("frustrated"));
        assert!(adjusted.contains("helpful"));
    }
}
