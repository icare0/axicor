use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysId {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoConfig {
    #[serde(default)]
    pub input: Vec<IoMatrix>,
    #[serde(default)]
    pub output: Vec<IoMatrix>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoMatrix {
    pub matrix_id_v1: SysId,
    pub name: String,
    pub entry_z: String, // "top", "mid", "bottom" or exact value
    #[serde(default)]
    pub pin: Vec<IoPin>, // In TOML this is [[input.pin]]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoPin {
    pub pin_id_v1: SysId,
    pub name: String,
    // VRAM Payload / C-ABI Size
    pub width: u32,
    pub height: u32,
    // Normalized UV Projection (0.0 .. 1.0)
    pub local_u: f32,
    pub local_v: f32,
    pub u_width: f32,
    pub v_height: f32,
    // Routing & Biology
    pub target_type: String,
    pub stride: u32,
    #[serde(default = "default_growth_steps")]
    pub growth_steps: u32,
    #[serde(default = "default_empty_pixel")]
    pub empty_pixel: String,
}

fn default_growth_steps() -> u32 {
    1000
}
fn default_empty_pixel() -> String {
    "skip".to_string()
}

impl IoConfig {
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
