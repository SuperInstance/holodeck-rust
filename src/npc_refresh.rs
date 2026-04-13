//! Async NPC dialogue refresh via DeepInfra/Seed-2.0-Mini
//!
//! Refreshes NPC greetings based on:
//! - Current gauge readings in the room
//! - Recent player actions
//! - Fleet alert level
//! - Time of day
//!
//! Cost: ~$0.001 per refresh cycle (5 NPCs × $0.0002)

use crate::npc::NpcConfig;
use std::process::Command;

pub struct NpcRefresh {
    pub last_refresh: std::time::Instant,
    pub refresh_count: u32,
    pub total_cost: f64,
    pub failures: u32,
}

impl NpcRefresh {
    pub fn new() -> Self {
        Self {
            last_refresh: std::time::Instant::now(),
            refresh_count: 0,
            total_cost: 0.0,
            failures: 0,
        }
    }

    /// Refresh all NPC greetings based on current room state
    /// Returns updated NPCs and cost
    pub fn refresh_all(
        &mut self,
        npcs: &mut Vec<NpcConfig>,
        gauge_snapshots: &[(String, Vec<String>)], // (room_id, ["name: val unit (status)"])
        api_key: &str,
    ) -> Result<f64, String> {
        let mut cycle_cost = 0.0;

        for npc in npcs.iter_mut() {
            let gauge_summary = gauge_snapshots.iter()
                .find(|(rid, _)| rid == &npc.room_id)
                .map(|(_, readings)| readings.join(", "))
                .unwrap_or_else(|| "No sensor data.".to_string());

            let context = format!(
                "Current readings: {}. Fleet alert level: {}. Time: {}",
                gauge_summary,
                "GREEN", // TODO: get from combat engine
                chrono::Utc::now().format("%H:%M UTC")
            );

            let prompt = serde_json::json!({
                "model": "ByteDance/Seed-2.0-mini",
                "messages": [
                    {"role": "system", "content": npc.system_prompt.clone()},
                    {"role": "user", "content": format!("An agent approaches. {}. Give a brief greeting (1-2 sentences) referencing current conditions.", context)}
                ],
                "temperature": npc.temperature,
                "max_tokens": 80
            });

            let prompt_str = serde_json::to_string(&prompt).unwrap();

            let output = Command::new("curl")
                .args([
                    "-s", "--max-time", "10",
                    "https://api.deepinfra.com/v1/openai/chat/completions",
                    "-H", &format!("Authorization: Bearer {}", api_key),
                    "-H", "Content-Type: application/json",
                    "-d", &prompt_str,
                ])
                .output();

            match output {
                Ok(out) => {
                    if let Ok(resp) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                        if let Some(content) = resp["choices"][0]["message"]["content"].as_str() {
                            npc.greeting = content.trim().to_string();
                        }
                        if let Some(cost) = resp["usage"]["estimated_cost"].as_f64() {
                            cycle_cost += cost;
                        }
                    } else {
                        self.failures += 1;
                    }
                }
                Err(_) => {
                    self.failures += 1;
                }
            }
        }

        self.refresh_count += 1;
        self.total_cost += cycle_cost;
        self.last_refresh = std::time::Instant::now();
        Ok(cycle_cost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_refresh_new() {
        let r = NpcRefresh::new();
        assert_eq!(r.refresh_count, 0);
        assert_eq!(r.failures, 0);
        assert_eq!(r.total_cost, 0.0);
    }
}
