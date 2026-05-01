//! SonarVision plugin for holodeck-rust
//!
//! Underwater room system with depth sonar physics.
//! Players explore ocean depths; sonar ping generates room descriptions.
//!
//! Room hierarchy:
//!   OceanSurface (0-5m) → WaterColumn (5-50m) → Seabed (50m+)

use crate::room::{Room, DataSource};
use crate::gauge::Gauge;

/// Water type classification (Jerlov water types simplified)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaterType {
    Coastal,
    Oceanic,
    Brackish,
    Polar,
}

impl WaterType {
    pub fn attenuation_coefficient(&self, depth: f64) -> f64 {
        match self {
            WaterType::Coastal => 0.2 + depth * 0.01,
            WaterType::Oceanic => 0.05 + depth * 0.002,
            WaterType::Brackish => 0.3 + depth * 0.015,
            WaterType::Polar => 0.1 + depth * 0.005,
        }
    }
}

/// A school of fish detected by sonar
#[derive(Debug, Clone)]
pub struct FishSchool {
    pub species: String,
    pub depth: f64,
    pub size: u32,
    pub biomass_kg: f64,
}

/// A feature on the seabed
#[derive(Debug, Clone)]
pub struct SeabedFeature {
    pub feature_type: String,
    pub depth: f64,
    pub description: String,
}

/// Underwater room state
#[derive(Debug, Clone)]
pub struct UnderwaterRoom {
    pub depth: f64,
    pub water_type: WaterType,
    pub visibility_m: f64,
    pub temperature_c: f64,
    pub fish_schools: Vec<FishSchool>,
    pub seabed_features: Vec<SeabedFeature>,
    pub bioluminescence: bool,
    pub ambient_light: f64,
    pub sonar_ping: Vec<f64>,
}

impl UnderwaterRoom {
    pub fn new(depth: f64, water_type: WaterType) -> Self {
        let ambient = (1.0 - depth / 100.0).max(0.05);
        let visibility = (15.0 - depth * 0.2).max(0.5);
        let temperature = 20.0 - depth * 0.35;
        let bioluminescence = depth > 30.0;

        let mut room = Self {
            depth,
            water_type,
            visibility_m: visibility,
            temperature_c: temperature,
            fish_schools: Vec::new(),
            seabed_features: Vec::new(),
            bioluminescence,
            ambient_light: ambient,
            sonar_ping: vec![0.0_f64; 32],
        };
        room.simulate_sonar_ping();
        room
    }

    pub fn generate_description(&self) -> String {
        let mut desc = String::new();
        desc.push_str(match self.depth as u32 {
            0..=5 => "The sunlit surface waters shimmer above you.\n",
            6..=20 => "The thermocline layer surrounds you.\n",
            21..=50 => "Deep blue twilight. Strange shapes in the darkness.\n",
            _ => "The abyssal plain stretches into infinite darkness.\n",
        });
        desc.push_str(match self.visibility_m as u32 {
            0..=2 => "Visibility is nearly zero.\n",
            3..=10 => "The water is murky.\n",
            _ => "The water is remarkably clear.\n",
        });
        for fish in &self.fish_schools {
            desc.push_str(&format!(
                "A school of {} ({} individuals) at {}m.\n",
                fish.species, fish.size, fish.depth
            ));
        }
        for feature in &self.seabed_features {
            desc.push_str(&format!("{}\n", feature.description));
        }
        if self.bioluminescence {
            desc.push_str("Bioluminescent sparks drift around you.\n");
        }
        desc
    }

    pub fn simulate_sonar_ping(&mut self) {
        let num_bins = self.sonar_ping.len();
        let mut returns = vec![0.0_f64; num_bins];
        let seabed_idx = (num_bins as f64 * 0.8) as usize;
        for i in 0..num_bins {
            let dist = (i as i32 - seabed_idx as i32).abs() as f64;
            returns[i] += (1.0 - dist / 8.0).max(0.0) * 0.9;
        }
        for fish in &self.fish_schools {
            let idx = ((fish.depth / 100.0) * num_bins as f64) as usize;
            if idx < num_bins {
                returns[idx] += (fish.size as f64 / 500.0).min(0.5);
            }
        }
        let atten = self.water_type.attenuation_coefficient(self.depth);
        for i in 0..num_bins {
            returns[i] *= (-atten * i as f64).exp();
        }
        self.sonar_ping = returns;
    }

    pub fn to_holodeck_room(&self, id: &str, name: &str) -> Room {
        let mut room = Room::new(id, name, &self.generate_description());
        room.data_source = DataSource::Sim;
        room.add_gauge(Gauge::new("depth", "m", 0.0, 100.0));
        room.add_gauge(Gauge::new("temperature", "°C", -5.0, 40.0));
        room.add_gauge(Gauge::new("visibility", "m", 0.0, 50.0));
        room.add_gauge(Gauge::new("ambient_light", "%", 0.0, 1.0));
        room.add_gauge(Gauge::new("bioluminescence", "", 0.0, 1.0));
        room
    }
}

/// Builder for underwater room networks
pub struct UnderwaterRoomBuilder;

impl UnderwaterRoomBuilder {
    pub fn build_ocean_surface(rooms: &mut Vec<Room>) -> String {
        let mut underwater = UnderwaterRoom::new(2.0, WaterType::Coastal);
        underwater.fish_schools.push(FishSchool {
            species: "Anchovy".into(), depth: 3.0, size: 200, biomass_kg: 10.0,
        });
        underwater.simulate_sonar_ping();
        let room = underwater.to_holodeck_room("ocean-surface", "Ocean Surface");
        let id = room.id.clone();
        rooms.push(room);
        id
    }

    pub fn build_water_column(rooms: &mut Vec<Room>, depth: f64, water_type: WaterType) -> String {
        let mut underwater = UnderwaterRoom::new(depth, water_type);
        match depth as u32 {
            0..=10 => underwater.fish_schools.push(FishSchool {
                species: "Herring".into(), depth: 8.0, size: 150, biomass_kg: 15.0,
            }),
            11..=30 => underwater.fish_schools.push(FishSchool {
                species: "Cod".into(), depth: 25.0, size: 50, biomass_kg: 40.0,
            }),
            _ => underwater.fish_schools.push(FishSchool {
                species: "Lanternfish".into(), depth, size: 500, biomass_kg: 5.0,
            }),
        }
        underwater.simulate_sonar_ping();
        let room = underwater.to_holodeck_room(
            &format!("water-column-{}", depth as u32),
            &format!("Water Column ({}m)", depth as u32),
        );
        let id = room.id.clone();
        rooms.push(room);
        id
    }

    pub fn build_seabed(rooms: &mut Vec<Room>, depth: f64) -> String {
        let mut underwater = UnderwaterRoom::new(depth, WaterType::Coastal);
        underwater.seabed_features.push(SeabedFeature {
            feature_type: "rock_formation".into(),
            depth,
            description: "A jagged rock formation rises from the seafloor, covered in anemones.".into(),
        });
        underwater.fish_schools.push(FishSchool {
            species: "Hagfish".into(), depth, size: 20, biomass_kg: 8.0,
        });
        underwater.simulate_sonar_ping();
        let room = underwater.to_holodeck_room(
            &format!("seabed-{}", depth as u32),
            &format!("Seabed ({}m)", depth as u32),
        );
        let id = room.id.clone();
        rooms.push(room);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_underwater_room_creation() {
        let room = UnderwaterRoom::new(15.0, WaterType::Coastal);
        assert_eq!(room.depth, 15.0);
        assert!(room.visibility_m > 0.0);
        assert!(room.temperature_c < 20.0);
    }

    #[test]
    fn test_sonar_ping_length() {
        let room = UnderwaterRoom::new(25.0, WaterType::Oceanic);
        assert_eq!(room.sonar_ping.len(), 32);
    }

    #[test]
    fn test_deeper_rooms_darker() {
        let shallow = UnderwaterRoom::new(5.0, WaterType::Coastal);
        let deep = UnderwaterRoom::new(80.0, WaterType::Oceanic);
        assert!(shallow.ambient_light > deep.ambient_light);
    }

    #[test]
    fn test_deep_bioluminescence() {
        let shallow = UnderwaterRoom::new(5.0, WaterType::Coastal);
        let deep = UnderwaterRoom::new(50.0, WaterType::Oceanic);
        assert!(!shallow.bioluminescence);
        assert!(deep.bioluminescence);
    }

    #[test]
    fn test_description_generated() {
        let room = UnderwaterRoom::new(25.0, WaterType::Oceanic);
        let desc = room.generate_description();
        assert!(!desc.is_empty());
    }

    #[test]
    fn test_holodeck_room_conversion() {
        let underwater = UnderwaterRoom::new(10.0, WaterType::Coastal);
        let room = underwater.to_holodeck_room("test-1", "Test Room");
        assert_eq!(room.gauges.len(), 5);
        assert!(room.gauges.contains_key("depth"));
    }

    #[test]
    fn test_underwater_builder() {
        let mut rooms = Vec::new();
        let surface_id = UnderwaterRoomBuilder::build_ocean_surface(&mut rooms);
        assert!(!surface_id.is_empty());
        assert_eq!(rooms.len(), 1);
    }

    #[test]
    fn test_full_underwater_exploration_chain() {
        let mut rooms = Vec::new();
        let surface = UnderwaterRoomBuilder::build_ocean_surface(&mut rooms);
        let column = UnderwaterRoomBuilder::build_water_column(&mut rooms, 25.0, WaterType::Oceanic);
        let seabed = UnderwaterRoomBuilder::build_seabed(&mut rooms, 50.0);
        assert_eq!(rooms.len(), 3);
        assert_ne!(surface, "");
        assert_ne!(column, "");
        assert_ne!(seabed, "");
    }
}
