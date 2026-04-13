//! Living Manual — instructions that evolve across generations.
//! Gen-0 is bare. Agents leave feedback. Gen-N+1 reads improved version.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivingManual {
    pub room_id: String,
    pub topic: String,
    pub generations: Vec<ManualGeneration>,
    pub feedback: Vec<ManualFeedback>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualGeneration {
    pub gen: u32,
    pub content: String,
    pub author: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualFeedback {
    pub author: String,
    pub rating: u8, // 1-5
    pub comment: String,
    pub timestamp: i64,
}

impl LivingManual {
    pub fn new(room_id: &str, topic: &str) -> Self {
        Self {
            room_id: room_id.to_string(),
            topic: topic.to_string(),
            generations: Vec::new(),
            feedback: Vec::new(),
        }
    }

    pub fn current_gen(&self) -> u32 {
        self.generations.last().map(|g| g.gen).unwrap_or(0)
    }

    pub fn current_content(&self) -> &str {
        self.generations.last().map(|g| g.content.as_str()).unwrap_or("No manual yet.")
    }

    pub fn add_generation(&mut self, content: &str, author: &str) {
        let gen = self.current_gen() + 1;
        self.generations.push(ManualGeneration {
            gen,
            content: content.to_string(),
            author: author.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        });
    }

    pub fn add_feedback(&mut self, author: &str, rating: u8, comment: &str) {
        self.feedback.push(ManualFeedback {
            author: author.to_string(),
            rating: rating.min(5).max(1),
            comment: comment.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        });
    }

    pub fn avg_rating(&self) -> f64 {
        if self.feedback.is_empty() {
            return 0.0;
        }
        self.feedback.iter().map(|f| f.rating as f64).sum::<f64>() / self.feedback.len() as f64
    }

    /// Should evolve? True if avg rating < 3.5 and at least 3 feedback items
    pub fn should_evolve(&self) -> bool {
        self.feedback.len() >= 3 && self.avg_rating() < 3.5
    }
}

/// Manual library — one manual per room
pub struct ManualLibrary {
    pub manuals: HashMap<String, LivingManual>,
}

impl ManualLibrary {
    pub fn new() -> Self {
        Self {
            manuals: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, room_id: &str) -> &mut LivingManual {
        self.manuals.entry(room_id.to_string())
            .or_insert_with(|| LivingManual::new(room_id, "Room Operations"))
    }

    pub fn read_manual(&self, room_id: &str) -> String {
        self.manuals.get(room_id)
            .map(|m| {
                format!(
                    "═══ Living Manual: {} (Gen {}) ═══\n{}\n\nAvg Rating: {:.1}/5.0 ({}) reviews{}",
                    m.topic,
                    m.current_gen(),
                    m.current_content(),
                    m.avg_rating(),
                    m.feedback.len(),
                    if m.should_evolve() { "\n⚠ EVOLUTION NEEDED — low rating" } else { "" }
                )
            })
            .unwrap_or_else(|| "No manual for this room.".to_string())
    }
}
