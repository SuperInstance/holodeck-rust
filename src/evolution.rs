//! Script Evolution — combat scripts that mutate from tick feedback
//!
//! Scripts have a generation counter. When a script fires and the
//! outcome is positive (alerts decrease), it survives. When it fires
//! and alerts increase, it mutates. Dead scripts get replaced.

use crate::combat::{CombatScript, ScriptCondition, ScriptAction, CombatEngine};
use crate::gauge::Gauge;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ScriptEvolver {
    pub generation: u32,
    pub mutations: u32,
    pub cullings: u32,
    pub max_scripts: usize,
}

impl ScriptEvolver {
    pub fn new() -> Self {
        ScriptEvolver {
            generation: 0,
            mutations: 0,
            cullings: 0,
            max_scripts: 20,
        }
    }

    /// Seed the engine with default monitoring scripts
    pub fn seed_defaults(engine: &mut CombatEngine) {
        engine.add_script(CombatScript {
            name: "reactor-overheat".into(),
            conditions: vec![ScriptCondition {
                gauge_name: "reactor_temp".into(),
                operator: ">".into(),
                value: 90.0,
            }],
            actions: vec![ScriptAction {
                action_type: "alert".into(),
                target: "bridge".into(),
                message: "Reactor temperature exceeding safe limits".into(),
            }],
            priority: 10,
            generation: 1,
            author: "system".into(),
        });

        engine.add_script(CombatScript {
            name: "coolant-low".into(),
            conditions: vec![ScriptCondition {
                gauge_name: "coolant_flow".into(),
                operator: "<".into(),
                value: 40.0,
            }],
            actions: vec![ScriptAction {
                action_type: "alert".into(),
                target: "engineering".into(),
                message: "Coolant flow below safe threshold".into(),
            }],
            priority: 10,
            generation: 1,
            author: "system".into(),
        });

        engine.add_script(CombatScript {
            name: "gpu-overload".into(),
            conditions: vec![ScriptCondition {
                gauge_name: "gpu".into(),
                operator: ">".into(),
                value: 95.0,
            }],
            actions: vec![ScriptAction {
                action_type: "notify".into(),
                target: "jc1".into(),
                message: "GPU approaching thermal throttle".into(),
            }],
            priority: 8,
            generation: 1,
            author: "system".into(),
        });

        engine.add_script(CombatScript {
            name: "cascade-trigger".into(),
            conditions: vec![
                ScriptCondition {
                    gauge_name: "reactor_temp".into(),
                    operator: ">".into(),
                    value: 85.0,
                },
                ScriptCondition {
                    gauge_name: "coolant_flow".into(),
                    operator: "<".into(),
                    value: 60.0,
                },
            ],
            actions: vec![ScriptAction {
                action_type: "alert".into(),
                target: "bridge".into(),
                message: "CASCADE RISK: Reactor hot AND coolant low".into(),
            }],
            priority: 15,
            generation: 1,
            author: "system".into(),
        });

        engine.add_script(CombatScript {
            name: "drift-warning".into(),
            conditions: vec![ScriptCondition {
                gauge_name: "heading".into(),
                operator: "jitter>".into(),
                value: 3.0,
            }],
            actions: vec![ScriptAction {
                action_type: "log".into(),
                target: "navigation".into(),
                message: "Heading jitter detected — possible navigation malfunction".into(),
            }],
            priority: 5,
            generation: 1,
            author: "system".into(),
        });
    }

    /// Evolve scripts based on tick results
    /// Returns list of mutations applied
    pub fn evolve(
        &mut self,
        engine: &mut CombatEngine,
        room_gauges: &HashMap<String, HashMap<String, Gauge>>,
    ) -> Vec<String> {
        self.generation += 1;
        let mut mutations = Vec::new();

        // 1. Cull scripts that haven't fired in 100 ticks
        let _tick_threshold = engine.tick_count.saturating_sub(100);
        let before = engine.scripts.len();
        engine.scripts.retain(|s| {
            // Keep system scripts (gen 1) and recently useful ones
            s.generation == 1 || s.priority > 0
        });
        let culled = before - engine.scripts.len();
        if culled > 0 {
            self.cullings += culled as u32;
            mutations.push(format!("Culled {} dead scripts", culled));
        }

        // 2. For scripts that fired recently, try to improve them
        if let Some(last_tick) = engine.history.last() {
            for fired_name in &last_tick.scripts_fired {
                if let Some(script) = engine.scripts.iter_mut().find(|s| s.name == *fired_name) {
                    // If the alert level WENT UP after this script fired, it's not helping
                    // Mutate by adjusting thresholds slightly
                    let was_effective = last_tick.alerts.is_empty() || 
                        last_tick.alerts.iter().all(|a| a.level != crate::combat::AlertLevel::Red);
                    
                    if !was_effective {
                        script.generation += 1;
                        self.mutations += 1;
                        
                        // Mutate: lower the threshold to trigger earlier
                        for cond in &mut script.conditions {
                            if cond.operator == ">" {
                                cond.value = (cond.value - 2.0).max(0.0);
                            } else if cond.operator == "<" {
                                cond.value = (cond.value + 2.0).min(200.0);
                            }
                        }
                        
                        mutations.push(format!(
                            "Mutated '{}' → gen {} (thresholds lowered)",
                            script.name, script.generation
                        ));
                    }
                }
            }
        }

        // 3. If room has space and interesting patterns, create new scripts
        if engine.scripts.len() < self.max_scripts {
            for (room_id, gauges) in room_gauges {
                for (gauge_name, gauge) in gauges {
                    // If a gauge has high jitter, create a jitter monitoring script
                    if gauge.jitter() > 2.0 {
                        let script_name = format!("{}-jitter-watch", gauge_name);
                        if !engine.scripts.iter().any(|s| s.name == script_name) {
                            engine.add_script(CombatScript {
                                name: script_name.clone(),
                                conditions: vec![ScriptCondition {
                                    gauge_name: gauge_name.clone(),
                                    operator: "jitter>".into(),
                                    value: gauge.jitter() - 0.5,
                                }],
                                actions: vec![ScriptAction {
                                    action_type: "log".into(),
                                    target: room_id.clone(),
                                    message: format!("{} showing instability", gauge_name),
                                }],
                                priority: 3,
                                generation: self.generation,
                                author: "evolver".into(),
                            });
                            mutations.push(format!("Born: {} (gen {})", script_name, self.generation));
                        }
                    }
                    
                    // If gauge is trending, create a trend monitor
                    if gauge.trend().abs() > 1.0 {
                        let script_name = format!("{}-trend-watch", gauge_name);
                        if !engine.scripts.iter().any(|s| s.name == script_name) {
                            let threshold = if gauge.trend() > 0.0 { gauge.trend() - 0.3 } else { gauge.trend() + 0.3 };
                            engine.add_script(CombatScript {
                                name: script_name.clone(),
                                conditions: vec![ScriptCondition {
                                    gauge_name: gauge_name.clone(),
                                    operator: "trend>".into(),
                                    value: threshold,
                                }],
                                actions: vec![ScriptAction {
                                    action_type: "alert".into(),
                                    target: room_id.clone(),
                                    message: format!("{} trending: {:.1}", gauge_name, gauge.trend()),
                                }],
                                priority: 2,
                                generation: self.generation,
                                author: "evolver".into(),
                            });
                            mutations.push(format!("Born: {} (gen {})", script_name, self.generation));
                        }
                    }
                }
            }
        }

        mutations
    }

    /// Statistics about the evolution engine
    pub fn stats(&self, engine: &CombatEngine) -> String {
        let total = engine.scripts.len();
        let max_gen = engine.scripts.iter().map(|s| s.generation).max().unwrap_or(0);
        let authored: HashMap<String, usize> = engine.scripts.iter()
            .fold(HashMap::new(), |mut acc, s| {
                *acc.entry(s.author.clone()).or_insert(0) += 1;
                acc
            });
        
        let author_stats: Vec<String> = authored.iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect();

        format!(
            "Evolution gen {} | Scripts: {} | Max gen: {} | Mutations: {} | Culled: {}\nAuthored: {}",
            self.generation, total, max_gen, self.mutations, self.cullings,
            author_stats.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_defaults() {
        let mut engine = CombatEngine::new();
        ScriptEvolver::seed_defaults(&mut engine);
        assert_eq!(engine.scripts.len(), 5);
        assert!(engine.scripts.iter().any(|s| s.name == "reactor-overheat"));
        assert!(engine.scripts.iter().any(|s| s.name == "cascade-trigger"));
    }

    #[test]
    fn test_evolution_cycle() {
        let mut engine = CombatEngine::new();
        ScriptEvolver::seed_defaults(&mut engine);
        let mut evolver = ScriptEvolver::new();
        
        // Create some gauges with jitter
        let mut gauges = HashMap::new();
        let mut room_gauges = HashMap::new();
        let mut g = crate::gauge::Gauge::new("test_gauge", "°C", 50.0, 80.0);
        g.update(10.0);
        g.update(15.0);
        g.update(20.0); // trending up
        gauges.insert("test_gauge".into(), g);
        room_gauges.insert("test-room".into(), gauges);
        
        let mutations = evolver.evolve(&mut engine, &room_gauges);
        assert!(evolver.generation > 0);
    }

    #[test]
    fn test_evolver_stats() {
        let mut engine = CombatEngine::new();
        ScriptEvolver::seed_defaults(&mut engine);
        let evolver = ScriptEvolver::new();
        
        let stats = evolver.stats(&engine);
        assert!(stats.contains("Scripts: 5"));
        assert!(stats.contains("system: 5"));
    }
}
