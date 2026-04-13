//! Async NPC dialogue refresh via DeepInfra/Seed-2.0-Mini
//!
//! Uses reqwest for non-blocking HTTP calls.
//! NPCs get refreshed with context-aware greetings based on live gauge data.

use crate::npc::NpcConfig;

#[derive(Debug, Clone)]
pub struct NpcRefresh {
    pub refresh_count: u32,
    pub total_cost: f64,
    pub failures: u32,
}

impl NpcRefresh {
    pub fn new() -> Self {
        Self {
            refresh_count: 0,
            total_cost: 0.0,
            failures: 0,
        }
    }
}

/// Call DeepInfra async and return (updated_npcs, cost, failures)
pub async fn refresh_npcs_async(
    mut npcs: Vec<NpcConfig>,
    gauge_snapshots: &[(String, Vec<String>)],
    api_key: &str,
) -> (Vec<NpcConfig>, f64, u32) {
    let client = reqwest::Client::new();
    let mut total_cost = 0.0;
    let mut failures = 0;

    for npc in npcs.iter_mut() {
        let gauge_summary = gauge_snapshots.iter()
            .find(|(rid, _)| rid == &npc.room_id)
            .map(|(_, readings)| readings.join(", "))
            .unwrap_or_else(|| "No sensor data.".to_string());

        let time_str = chrono::Utc::now().format("%H:%M UTC").to_string();
        let context = format!(
            "Current readings: {}. Fleet alert: GREEN. Time: {}",
            gauge_summary, time_str
        );

        let body = serde_json::json!({
            "model": "ByteDance/Seed-2.0-mini",
            "messages": [
                {"role": "system", "content": npc.system_prompt.clone()},
                {"role": "user", "content": format!("An agent approaches. {}. Give a brief greeting (1-2 sentences) referencing current conditions.", context)}
            ],
            "temperature": npc.temperature,
            "max_tokens": 80
        });

        match client
            .post("https://api.deepinfra.com/v1/openai/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(content) = data["choices"][0]["message"]["content"].as_str() {
                        npc.greeting = content.trim().to_string();
                    } else {
                        failures += 1;
                    }
                    if let Some(cost) = data["usage"]["estimated_cost"].as_f64() {
                        total_cost += cost;
                    }
                } else {
                    failures += 1;
                }
            }
            Err(_) => {
                failures += 1;
            }
        }
    }

    (npcs, total_cost, failures)
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
