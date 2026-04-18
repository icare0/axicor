use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Full `anatomy.toml`  list of zone layers.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AnatomyConfig {
    #[serde(rename = "layer")]
    pub layers: Vec<LayerConfig>,
}

/// A single [[layer]] block from anatomy.toml.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LayerConfig {
    /// Layer name, e.g., "L1", "L4", "Nuclear".
    pub name: String,
    /// Layer height as a fraction of world.height_um (0.0..1.0).
    pub height_pct: f32,
    /// [DOD FIX] Absolute voxel density.
    pub density: f32,
    /// Strict quotas: {type_name  fraction}. Sum must be 1.0.
    pub composition: HashMap<String, f32>,
}

impl AnatomyConfig {
    /// Parses config from TOML string.
    pub fn parse(src: &str) -> Result<Self, String> {
        toml::from_str(src).map_err(|e| format!("TOML parse error: {}", e))
    }

    /// Loads config from disk.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;
        Self::parse(&content)
    }
}

#[cfg(test)]
#[path = "test_anatomy.rs"]
mod test_anatomy;
