//! Gauge — live sensor readings in rooms.
//! A gauge has a name, value, thresholds, and history.
//! Gauges are the bridge between real hardware and the MUD.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const HISTORY_LEN: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gauge {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub yellow_threshold: f64,
    pub red_threshold: f64,
    pub history: VecDeque<f64>,
}

impl Gauge {
    pub fn new(name: &str, unit: &str, yellow: f64, red: f64) -> Self {
        Self {
            name: name.to_string(),
            value: 0.0,
            unit: unit.to_string(),
            yellow_threshold: yellow,
            red_threshold: red,
            history: VecDeque::with_capacity(HISTORY_LEN),
        }
    }

    pub fn update(&mut self, value: f64) {
        self.history.push_back(self.value);
        if self.history.len() > HISTORY_LEN {
            self.history.pop_front();
        }
        self.value = value;
    }

    pub fn status(&self) -> GaugeStatus {
        let abs = self.value.abs();
        if abs >= self.red_threshold {
            GaugeStatus::Red
        } else if abs >= self.yellow_threshold {
            GaugeStatus::Yellow
        } else {
            GaugeStatus::Green
        }
    }

    /// Rate of change over last N readings (jitter detection)
    pub fn jitter(&self) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }
        let recent: Vec<f64> = self.history.iter().rev().take(10).copied().collect();
        let mut deltas = 0.0;
        for i in 0..recent.len() - 1 {
            deltas += (recent[i] - recent[i + 1]).abs();
        }
        deltas / (recent.len() - 1) as f64
    }

    /// Trend: positive = rising, negative = falling
    pub fn trend(&self) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }
        let recent: Vec<f64> = self.history.iter().rev().take(20).copied().collect();
        if recent.len() < 2 {
            return 0.0;
        }
        (recent.first().copied().unwrap_or(0.0) - recent.last().copied().unwrap_or(0.0)) / recent.len() as f64
    }

    pub fn display(&self) -> String {
        let status_char = match self.status() {
            GaugeStatus::Green => '.',
            GaugeStatus::Yellow => '~',
            GaugeStatus::Red => '!',
        };
        let jitter_indicator = if self.jitter() > self.value * 0.1 { "⚡" } else { " " };
        let trend_indicator = if self.trend() > 0.01 { "↑" } else if self.trend() < -0.01 { "↓" } else { "→" };
        format!(
            "  {} {}: {:.2}{} [{}] {}{}",
            trend_indicator, self.name, self.value, self.unit,
            status_char, jitter_indicator,
            match self.status() {
                GaugeStatus::Green => "",
                GaugeStatus::Yellow => " WARNING",
                GaugeStatus::Red => " CRITICAL",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GaugeStatus {
    Green,
    Yellow,
    Red,
}
