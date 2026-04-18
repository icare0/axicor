/// Configuration architecture (Spec 02 1.1).
///
/// `simulation.toml`  global "Laws of the Universe".
/// The single parser for the engine (both baker and runtime use this).
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub mod anatomy;
pub mod blueprints;
pub mod brain;
pub mod instance;
pub mod io;
pub mod manifest;
pub mod sys;

pub use anatomy::{AnatomyConfig, LayerConfig};
pub use blueprints::{BlueprintsConfig, NeuronType};
pub use instance::InstanceConfig;
pub use io::{IoConfig, IoMatrix, IoPin};
pub use manifest::*;

/// Full parsed `simulation.toml` config.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SimulationConfig {
    #[serde(default)]
    pub model_id_v1: Option<crate::config::sys::SystemMeta>,

    pub world: WorldConfig,

    /// [DOD FIX] Versioned simulation block.
    /// In source  [sim_v_1], in archive  [manifest_sim_v_1].
    #[serde(alias = "simulation")]
    #[serde(alias = "manifest_sim_v_1")]
    #[serde(rename = "sim_v_1")]
    pub simulation: SimulationParams,

    /// [DOD FIX] List of departments (Brains) attached to this model.
    #[serde(rename = "department", default)]
    pub departments: Vec<DepartmentEntry>,

    /// [DOD FIX] Inter-department (Father-level) connections at the model level (Grandfather).
    #[serde(rename = "connection", default)]
    pub connections: Vec<crate::config::brain::ConnectionEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DepartmentEntry {
    #[serde(default)]
    pub depart_id_v1: Option<crate::config::sys::SystemMeta>,
    pub name: String,
    pub config: std::path::PathBuf,
}

/// Physical world dimensions (1.1).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorldConfig {
    pub width_um: u32,
    pub depth_um: u32,
    pub height_um: u32,
}

/// Global simulation parameters (1.1).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SimulationParams {
    pub tick_duration_us: u32,
    pub total_ticks: u64,
    pub master_seed: String,
    pub voxel_size_um: f32,
    pub signal_speed_m_s: f32,
    pub sync_batch_ticks: u32,

    #[serde(default = "default_segment_length")]
    pub segment_length_voxels: u32,

    #[serde(default = "default_max_steps")]
    pub axon_growth_max_steps: u32,
}

fn default_segment_length() -> u32 {
    5
}
fn default_max_steps() -> u32 {
    2000
}

impl SimulationConfig {
    /// Total voxel count for a given voxel size (in m).
    pub fn total_voxels(&self) -> u64 {
        let v_um = self.simulation.voxel_size_um;
        if v_um <= 0.0 {
            return 0;
        }
        let w = (self.world.width_um as f32 / v_um) as u64;
        let d = (self.world.depth_um as f32 / v_um) as u64;
        let h = (self.world.height_um as f32 / v_um) as u64;
        w * d * h
    }
    /// Parses config from TOML string.
    pub fn parse(src: &str) -> Result<Self, String> {
        toml::from_str(src).map_err(|e| format!("TOML parse error: {}", e))
    }

    /// Loads config from disk.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;
        Self::parse(&content)
    }
}

#[cfg(test)]
#[path = "test_config.rs"]
mod test_config;
